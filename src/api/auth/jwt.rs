use actix_web::{
    dev::Payload,
    error::ErrorUnauthorized,
    http::header::{self, HeaderMap},
    web, Error, FromRequest, HttpRequest,
};
use jsonwebtoken::{decode, decode_header, Algorithm, Validation};
use serde::{Deserialize, Serialize};
use log::error;
use crate::api::auth::jwks::BNETJwksService;
use crate::dal::Dal;

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

pub async fn validate_token(token: &str, jwks_service: &BNETJwksService) -> Result<Claims, Box<dyn std::error::Error + Send + Sync>> {
    let header = decode_header(token)?;
    let kid = header.kid.ok_or("No kid found in token header")?;

    let decoding_key = jwks_service.get_decoding_key(&kid).await?;
    
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[""]); // BattleNet might need client_id here if it's in the 'aud' claim
    // Disable audience check if we don't know the client_id yet or if it's not present in OIDC id_token from BNet in a way we want to strictly enforce here without config
    validation.validate_aud = false; 

    let token_data = decode::<Claims>(token, &decoding_key, &validation)?;

    Ok(token_data.claims)
}

impl FromRequest for AuthenticatedUser {
    type Error = Error;
    type Future = futures_util::future::LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut Payload) -> Self::Future {
        let req = req.clone();
        Box::pin(async move {
            let jwks_service = req.app_data::<web::Data<BNETJwksService>>()
                .ok_or_else(|| {
                    error!("JwksService not found in app data");
                    ErrorUnauthorized("Authentication service unavailable")
                })?;

            let db = req.app_data::<web::Data<std::sync::Arc<Dal>>>()
                .ok_or_else(|| {
                    error!("Dal not found in app data");
                    ErrorUnauthorized("Authentication service unavailable")
                })?;

            let token = extract_token_from_auth_header(req.headers())
                .map_err(ErrorUnauthorized)?;

            let claims = validate_token(&token, jwks_service).await.map_err(|e| {
                error!("Token validation failed: {:?}", e);
                ErrorUnauthorized("Invalid token")
            })?;

            let user_id = claims.sub.parse::<i64>().map_err(|_| {
                ErrorUnauthorized("Invalid user id")
            })?;

            let user = db.users
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

            Ok(AuthenticatedUser {
                id: claims.sub,
                battletag: claims.battle_tag,
                roles,
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
