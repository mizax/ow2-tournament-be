use base64::{Engine as _, engine::general_purpose::STANDARD};
use jsonwebtoken::DecodingKey;
use log::info;
use serde::Deserialize;
use std::sync::RwLock;
use std::time::{Duration, Instant};

#[derive(Debug, Deserialize, Clone)]
pub struct Jwk {
    pub kid: String,
    pub n: String,
    pub e: String,
}

#[derive(Debug, Deserialize)]
struct JwksResponse {
    keys: Vec<Jwk>,
}

pub struct BNETJwksService {
    jwks_url: String,
    keys: RwLock<Option<(Vec<Jwk>, Instant)>>,
    cache_duration: Duration,
}

impl BNETJwksService {
    pub fn new(jwks_url: String, cache_duration: Duration) -> Self {
        Self {
            jwks_url,
            keys: RwLock::new(None),
            cache_duration,
        }
    }

    pub async fn get_keys(&self) -> Result<Vec<Jwk>, Box<dyn std::error::Error + Send + Sync>> {
        {
            let read_guard = self
                .keys
                .read()
                .map_err(|_| "Failed to acquire JWKS read lock")?;
            if let Some((keys, last_updated)) = &*read_guard {
                if last_updated.elapsed() < self.cache_duration {
                    return Ok(keys.clone());
                }
            }
        }

        self.refresh_keys().await
    }

    pub async fn refresh_keys(&self) -> Result<Vec<Jwk>, Box<dyn std::error::Error + Send + Sync>> {
        info!("Refreshing JWKS from {}", self.jwks_url);
        let response: JwksResponse = reqwest::get(&self.jwks_url).await?.json().await?;
        let keys = response.keys;

        {
            let mut write_guard = self
                .keys
                .write()
                .map_err(|_| "Failed to acquire JWKS write lock")?;
            *write_guard = Some((keys.clone(), Instant::now()));
        }

        Ok(keys)
    }

    pub async fn get_decoding_key(
        &self,
        kid: &str,
    ) -> Result<DecodingKey, Box<dyn std::error::Error + Send + Sync>> {
        let keys = self.get_keys().await?;
        let jwk = keys
            .iter()
            .find(|k| k.kid == kid)
            .ok_or_else(|| format!("JWK with kid {} not found", kid))?;

        // Battle.net: n & e — ordinary Base64, not RFC 7517 / 7518 compliant
        let n_bytes = STANDARD.decode(&jwk.n)?;
        let e_bytes = STANDARD.decode(&jwk.e)?;

        Ok(DecodingKey::from_rsa_raw_components(&n_bytes, &e_bytes))
    }
}
