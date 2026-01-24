use actix_web::{HttpRequest, HttpResponse, Responder, post, web, get};
use actix_web::http::header::USER_AGENT;
use serde::Deserialize;
use std::sync::Arc;
use crate::api::auth::jwt::AuthenticatedUser;
use crate::api::error::ApiError;
use crate::dal::{Dal, NewRegistration, RegistrationStatus, RoleValue};

#[derive(Deserialize)]
pub struct RegistrationRequest {
    #[serde(rename = "altAccounts")]
    pub alt_accounts: Option<Vec<String>>,
    pub twitch: String,
    pub discord: String,
    #[serde(rename = "primaryRole")]
    pub primary_role: Option<RoleValue>,
    #[serde(rename = "secondaryRole")]
    pub secondary_role: Option<RoleValue>,
    pub guarantors: Option<Vec<String>>,
    #[serde(rename = "additionalInfo")]
    pub additional_info: String,
    #[serde(rename = "rulesAccepted")]
    pub rules_accepted: bool,
}

#[derive(serde::Serialize)]
pub struct RegistrationResponse {
    pub registration_id: i64,
    pub status: RegistrationStatus,
}

#[derive(serde::Serialize)]
pub struct RegistrationStatusResponse {
    pub status: Option<RegistrationStatus>,
    pub request_id: Option<i64>,
}

#[post("/{sef_uri}/register")]
pub async fn register(
    user: AuthenticatedUser,
    db: web::Data<Arc<Dal>>,
    sef_uri: web::Path<String>,
    payload: web::Json<RegistrationRequest>,
    req: HttpRequest,
) -> actix_web::Result<impl Responder, ApiError> {
    let mut validation_errors = Vec::new();
    if payload.twitch.trim().is_empty() {
        validation_errors.push("validation.twitch.required".to_string());
    }
    if payload.discord.trim().is_empty() {
        validation_errors.push("validation.discord.required".to_string());
    }
    if payload.primary_role.is_none() {
        validation_errors.push("validation.primary_role.required".to_string());
    }
    if !payload.rules_accepted {
        validation_errors.push("validation.rules_accepted.required".to_string());
    }
    if let Some(primary_role) = payload.primary_role {
        if primary_role != RoleValue::Flex && payload.secondary_role.is_none() {
            validation_errors.push("validation.secondary_role.required".to_string());
        }
        if let Some(secondary_role) = payload.secondary_role {
            if primary_role == secondary_role {
                validation_errors.push("validation.secondary_role.same_as_primary".to_string());
            }
        }
    }
    if !validation_errors.is_empty() {
        return Err(ApiError::ValidationError {
            errors: validation_errors,
        });
    }

    let user_id = user.id.parse::<i64>().map_err(|_| ApiError::BadRequest {
        error: "Invalid user id".to_string(),
        details: "User id from auth token is not a number.".to_string(),
    })?;

    let tournament = db
        .tournaments
        .get_by_sef_title(&sef_uri)
        .await
        .map_err(|e| {
            log::error!("Failed to load tournament {}: {}", sef_uri, e);
            ApiError::InternalError { error: e.to_string() }
        })?
        .ok_or(ApiError::NotFound)?;

    let ip_address = req
        .connection_info()
        .realip_remote_addr()
        .map(str::to_string)
        .or_else(|| req.peer_addr().map(|addr| addr.ip().to_string()))
        .unwrap_or_else(|| "unknown".to_string());

    let user_agent = req
        .headers()
        .get(USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_string();

    db.users
        .upsert_battletag(user_id, &user.battletag)
        .await
        .map_err(|e| {
            log::error!("Failed to upsert battletag for user {}: {}", user_id, e);
            ApiError::InternalError { error: e.to_string() }
        })?;

    let user_battletag_id = db
        .users
        .find_battletag_id(user_id, &user.battletag)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch battletag id for user {}: {}", user_id, e);
            ApiError::InternalError { error: e.to_string() }
        })?
        .ok_or(ApiError::InternalError {
            error: "User battletag not found after insert.".to_string(),
        })?;

    let already_registered = db
        .registrations
        .active_registration_exists(tournament.id, user_id)
        .await
        .map_err(|e| {
            log::error!(
                "Failed to check registration for user {} in tournament {}: {}",
                user_id,
                tournament.id,
                e
            );
            ApiError::InternalError { error: e.to_string() }
        })?;

    if already_registered {
        return Err(ApiError::BadRequest {
            error: "already_registered".to_string(),
            details: "User already has an active registration for this tournament.".to_string(),
        });
    }

    let alt_accounts_json = serde_json::to_string(
        payload.alt_accounts.as_deref().unwrap_or(&[]),
    )
        .map_err(|e| ApiError::InternalError {
            error: e.to_string(),
        })?;
    let guarantors_json = serde_json::to_string(
        payload.guarantors.as_deref().unwrap_or(&[]),
    )
        .map_err(|e| ApiError::InternalError {
            error: e.to_string(),
        })?;

    let registration = NewRegistration {
        tournament_id: tournament.id,
        user_id,
        user_battletag_id,
        status: RegistrationStatus::Pending,
        alt_accounts_json,
        twitch: payload.twitch.clone(),
        discord: payload.discord.clone(),
        primary_role: payload.primary_role,
        secondary_role: payload.secondary_role,
        guarantors_json,
        additional_info: payload.additional_info.clone(),
        rules_accepted: payload.rules_accepted,
        ip_address,
        user_agent,
        decline_reason: None,
    };

    let registration_id = db.registrations
        .create(registration)
        .await
        .map_err(|e| {
            log::error!(
                "Failed to create registration for user {} in tournament {}: {}",
                user_id,
                tournament.id,
                e
            );
            ApiError::InternalError { error: e.to_string() }
        })?;

    Ok(HttpResponse::Created().json(RegistrationResponse {
        registration_id,
        status: RegistrationStatus::Pending,
    }))
}

#[get("/{sef_uri}/registration-status")]
pub async fn registration_status(
    user: AuthenticatedUser,
    db: web::Data<Arc<Dal>>,
    sef_uri: web::Path<String>,
) -> actix_web::Result<impl Responder, ApiError> {
    let user_id = user.id.parse::<i64>().map_err(|_| ApiError::BadRequest {
        error: "Invalid user id".to_string(),
        details: "User id from auth token is not a number.".to_string(),
    })?;

    let tournament = db
        .tournaments
        .get_by_sef_title(&sef_uri)
        .await
        .map_err(|e| {
            log::error!("Failed to load tournament {}: {}", sef_uri, e);
            ApiError::InternalError { error: e.to_string() }
        })?
        .ok_or(ApiError::NotFound)?;

    let request = db
        .registrations
        .find_user_registration(user_id, tournament.id)
        .await
        .map_err(|e| {
            log::error!("Failed to load registration for user {} in tournament {}: {}", user_id, tournament.id, e);
            ApiError::InternalError { error: e.to_string() }
        })?;

    let response = match request {
        Some(request) => RegistrationStatusResponse { status: Some(request.status), request_id: Some(request.id) },
        None => RegistrationStatusResponse { status: None, request_id: None },
    };

    Ok(HttpResponse::Ok().json(response))
}
