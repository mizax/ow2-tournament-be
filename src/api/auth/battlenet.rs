use actix_web::{get, web, HttpResponse, Responder, Result};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use crate::api::auth::jwt::UserRole;
use crate::api::auth::state_store::OAuthStateStore;
use crate::api::error::ApiError;
use crate::config::Config;
use crate::dal::Dal;

#[derive(Serialize, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id_token: Option<String>,
}

#[derive(Deserialize)]
pub struct BattleNetUserInfo {
    pub id: u64,
    pub battletag: String,
}

#[derive(Serialize)]
pub struct User {
    pub id: u64,
    pub battletag: String,
    pub roles: Vec<UserRole>,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub user: User,
    pub id_token: Option<String>,
}

const BATTLE_NET_AUTH_URL: &str = "https://oauth.battle.net/authorize";
const BATTLE_NET_TOKEN_URL: &str = "https://oauth.battle.net/token";
const BATTLE_NET_USERINFO_URL: &str = "https://oauth.battle.net/userinfo";

#[derive(Deserialize)]
pub struct AuthRequest {
    pub region: Option<String>,
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

#[get("/battlenet")]
pub async fn auth(
    req: actix_web::HttpRequest,
    query: web::Query<AuthRequest>,
    config: web::Data<Arc<Config>>,
    state_store: web::Data<OAuthStateStore>,
) -> Result<impl Responder> {
    let state: String = rand::rng()
        .sample_iter(&rand::distr::Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    state_store.add_state(state.clone());

    let host = req
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let base_redirect_host = if host == config.app.alt_host.replace("http://", "").replace("https://", "") {
        &config.app.alt_host
    } else {
        &config.app.main_host
    };

    let redirect_uri = format!("{}{}", base_redirect_host, config.battlenet.redirect_uri);

    let mut params = HashMap::new();
    params.insert("client_id", config.battlenet.client_id.clone());
    params.insert("redirect_uri", redirect_uri);
    params.insert("response_type", "code".to_string());
    params.insert("scope", "openid".to_string());
    params.insert("state", state);

    let base_url = match query.region.as_deref() {
        Some("cn") => "https://oauth.battlenet.com.cn/authorize",
        _ => BATTLE_NET_AUTH_URL,
    };

    let auth_url = format!(
        "{}?{}",
        base_url,
        serde_urlencoded::to_string(&params).map_err(|e| ApiError::InternalError { error: e.to_string() })?
    );

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "auth_url": auth_url,
    })))
}

#[get("/battlenet/callback")]
pub async fn callback(
    req: actix_web::HttpRequest,
    query: web::Query<CallbackQuery>,
    config: web::Data<Arc<Config>>,
    state_store: web::Data<OAuthStateStore>,
    db: web::Data<Arc<Dal>>,
) -> Result<impl Responder> {
    if let Some(error) = &query.error {
        return Err(ApiError::BadRequest {
            error: error.clone(),
            details: query.error_description.clone().unwrap_or_default(),
        }.into());
    }

    let state = query.state.as_ref().ok_or_else(|| ApiError::BadRequest {
        error: "Missing state".to_string(),
        details: "State parameter is required".to_string(),
    })?;

    if state_store.verify_and_remove_state(state).is_none() {
        return Err(ApiError::BadRequest {
            error: "Invalid state".to_string(),
            details: "State parameter is invalid or expired".to_string(),
        }.into());
    }

    let code = query.code.as_ref().ok_or_else(|| ApiError::BadRequest {
        error: "Missing code".to_string(),
        details: "Authorization code is required".to_string(),
    })?;

    let host = req
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    let base_redirect_host = if host == config.app.alt_host.replace("http://", "").replace("https://", "") {
        &config.app.alt_host
    } else {
        &config.app.main_host
    };

    let redirect_uri = format!("{}{}", base_redirect_host, config.battlenet.redirect_uri);

    let client = reqwest::Client::new();
    
    let token_response = client
        .post(BATTLE_NET_TOKEN_URL)
        .basic_auth(&config.battlenet.client_id, Some(&config.battlenet.client_secret))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", &redirect_uri),
        ])
        .send()
        .await
        .map_err(|e| ApiError::InternalError { error: format!("Failed to exchange token: {}", e) })?;

    if !token_response.status().is_success() {
        let error_text = token_response.text().await.unwrap_or_default();
        return Err(ApiError::InternalError {
            error: format!("Token exchange failed: {}", error_text),
        }.into());
    }

    let tokens: TokenResponse = token_response
        .json()
        .await
        .map_err(|e| ApiError::InternalError { error: format!("Failed to parse token response: {}", e) })?;

    let userinfo_response = client
        .get(BATTLE_NET_USERINFO_URL)
        .bearer_auth(&tokens.access_token)
        .send()
        .await
        .map_err(|e| ApiError::InternalError { error: format!("Failed to fetch userinfo: {}", e) })?;

    if !userinfo_response.status().is_success() {
        let error_text = userinfo_response.text().await.unwrap_or_default();
        return Err(ApiError::InternalError {
            error: format!("Userinfo request failed: {}", error_text),
        }.into());
    }

    let bnet_user: BattleNetUserInfo = userinfo_response
        .json()
        .await
        .map_err(|e| ApiError::InternalError { error: format!("Failed to parse userinfo response: {}", e) })?;

    let user_id = i64::try_from(bnet_user.id)
        .map_err(|_| ApiError::InternalError { error: "Battle.net user id out of range".to_string() })?;

    let existing_user = db.users
        .find_by_id(user_id)
        .await
        .map_err(|e| ApiError::InternalError { error: format!("Failed to fetch user: {}", e) })?;

    if existing_user.is_none() {
        db.users
            .create_user(user_id)
            .await
            .map_err(|e| ApiError::InternalError { error: format!("Failed to create user: {}", e) })?;

        db.users
            .insert_battletag(user_id, &bnet_user.battletag)
            .await
            .map_err(|e| ApiError::InternalError { error: format!("Failed to insert battletag: {}", e) })?;
    } else {
        let battletag_exists = db.users
            .battletag_exists(user_id, &bnet_user.battletag)
            .await
            .map_err(|e| ApiError::InternalError { error: format!("Failed to check battletag: {}", e) })?;

        if !battletag_exists {
            db.users
                .insert_battletag(user_id, &bnet_user.battletag)
                .await
                .map_err(|e| ApiError::InternalError { error: format!("Failed to insert battletag: {}", e) })?;

            db.users
                .touch_updated_at(user_id)
                .await
                .map_err(|e| ApiError::InternalError { error: format!("Failed to update user timestamp: {}", e) })?;
        }
    }

    let mut roles = vec![UserRole::NormalUser];
    if existing_user.as_ref().map(|user| user.is_admin).unwrap_or(false) {
        roles.push(UserRole::Admin);
    }

    let is_manager = db.tournament_managers
        .user_is_manager(user_id)
        .await
        .map_err(|e| ApiError::InternalError { error: format!("Failed to check tournament manager status for user {}: {}", user_id, e) })?;
    if is_manager {
        roles.push(UserRole::TournamentManager);
    }

    let response = AuthResponse {
        user: User {
            id: bnet_user.id,
            battletag: bnet_user.battletag,
            roles,
        },
        id_token: tokens.id_token,
    };

    Ok(HttpResponse::Ok().json(response))
}
