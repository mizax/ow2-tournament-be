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
    pub roles: Vec<String>,
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

            let token = extract_token_from_auth_header(req.headers())
                .map_err(ErrorUnauthorized)?;

            let claims = validate_token(&token, jwks_service).await.map_err(|e| {
                error!("Token validation failed: {:?}", e);
                ErrorUnauthorized("Invalid token")
            })?;

            Ok(AuthenticatedUser {
                id: claims.sub,
                battletag: claims.battle_tag,
                roles: vec![],
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
