use crate::api::auth::jwks::BNETJwksService;
use crate::config::Config;
use crate::dal::Dal;
use actix_web::{
    Error, FromRequest, HttpRequest,
    dev::Payload,
    error::ErrorUnauthorized,
    http::header::{self, HeaderMap},
    web,
};
use jsonwebtoken::{Algorithm, Validation, decode, decode_header};
use log::error;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub battle_tag: String,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub id: String,
    pub battletag: String,
    pub roles: Vec<UserRole>,
    pub authorities: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, sqlx::Type, PartialEq)]
pub enum UserRole {
    #[serde(rename = "admin")]
    Admin,
    #[serde(rename = "tournament_manager")]
    TournamentManager,
    #[serde(rename = "normal_user")]
    NormalUser,
}

impl AuthenticatedUser {
    pub fn has_authority(&self, authority: &str) -> bool {
        self.is_admin() || self.authorities.iter().any(|a| a == authority)
    }

    pub fn is_admin(&self) -> bool {
        self.roles.contains(&UserRole::Admin)
    }
}

pub async fn validate_token(
    token: &str,
    jwks_service: &BNETJwksService,
    client_id: &str,
) -> Result<Claims, Box<dyn std::error::Error + Send + Sync>> {
    let header = decode_header(token)?;
    let kid = header.kid.ok_or("No kid found in token header")?;

    let decoding_key = jwks_service.get_decoding_key(&kid).await?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[client_id]);

    let token_data = decode::<Claims>(token, &decoding_key, &validation)?;

    Ok(token_data.claims)
}

impl FromRequest for AuthenticatedUser {
    type Error = Error;
    type Future = futures_util::future::LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req = req.clone();
        Box::pin(async move {
            let jwks_service = req
                .app_data::<web::Data<BNETJwksService>>()
                .ok_or_else(|| {
                    error!("JwksService not found in app data");
                    ErrorUnauthorized("Authentication service unavailable")
                })?;

            let config = req
                .app_data::<web::Data<Arc<Config>>>()
                .ok_or_else(|| {
                    error!("Config not found in app data");
                    ErrorUnauthorized("Authentication service unavailable")
                })?;

            let db = req
                .app_data::<web::Data<std::sync::Arc<Dal>>>()
                .ok_or_else(|| {
                    error!("Dal not found in app data");
                    ErrorUnauthorized("Authentication service unavailable")
                })?;

            let token = extract_token_from_auth_header(req.headers()).map_err(ErrorUnauthorized)?;

            let claims = validate_token(&token, jwks_service, &config.battlenet.client_id)
                .await
                .map_err(|e| {
                    error!("Token validation failed: {:?}", e);
                    ErrorUnauthorized("Invalid token")
                })?;

            let user_id = claims
                .sub
                .parse::<i64>()
                .map_err(|_| ErrorUnauthorized("Invalid user id"))?;

            let user = db
                .users
                .find_by_id(user_id)
                .await
                .map_err(|e| {
                    error!("Failed to fetch user {}: {:?}", user_id, e);
                    ErrorUnauthorized("Invalid user")
                })?
                .ok_or_else(|| ErrorUnauthorized("User not found"))?;

            if user.is_banned {
                return Err(ErrorUnauthorized("User is banned"));
            }

            let mut roles = vec![UserRole::NormalUser];
            if user.is_admin {
                roles.push(UserRole::Admin);
            }

            let is_manager = db
                .tournament_managers
                .user_is_manager(user_id)
                .await
                .map_err(|e| {
                    error!(
                        "Failed to check tournament manager status for user {}: {:?}",
                        user_id, e
                    );
                    ErrorUnauthorized("Invalid user")
                })?;
            if is_manager {
                roles.push(UserRole::TournamentManager);
            }

            let authorities = db
                .users
                .list_authorities_for_user(user_id)
                .await
                .map_err(|e| {
                    error!("Failed to load authorities for user {}: {:?}", user_id, e);
                    ErrorUnauthorized("Invalid user")
                })?;

            Ok(AuthenticatedUser {
                id: claims.sub,
                battletag: claims.battle_tag,
                roles,
                authorities,
            })
        })
    }
}

fn extract_token_from_auth_header(headers: &HeaderMap) -> Result<String, &'static str> {
    let header = headers
        .get(header::AUTHORIZATION)
        .ok_or("Authorization header missing")?
        .to_str()
        .map_err(|_| "Authorization header is not a valid string")?;

    if !header.starts_with("Bearer ") {
        return Err("Authorization header must be a Bearer token");
    }

    Ok(header[7..].to_string())
}
