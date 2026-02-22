use crate::api::ApiError;
use reqwest::Client;
use reqwest::StatusCode;
use serde::Deserialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

const TOKEN_EXPIRY_SAFETY_BUFFER: Duration = Duration::from_secs(60);
const DEFAULT_TWITCH_TOKEN_URL: &str = "https://id.twitch.tv/oauth2/token";
const DEFAULT_TWITCH_STREAMS_URL: &str = "https://api.twitch.tv/helix/streams";

#[derive(Clone)]
pub struct TwitchApiService {
    client_id: String,
    client_secret: String,
    token_url: String,
    streams_url: String,
    client: Client,
    app_access_token: Arc<Mutex<Option<CachedAppAccessToken>>>,
}

#[derive(Clone)]
pub struct TwitchLiveStream {
    pub user_login: String,
    pub user_name: String,
    pub title: String,
    pub viewer_count: i64,
    pub started_at: String,
    pub game_name: String,
    pub thumbnail_url: String,
}

#[derive(Debug, Deserialize)]
struct TwitchAppAccessTokenResponse {
    access_token: String,
    expires_in: u64,
    token_type: String,
}

#[derive(Debug, Deserialize)]
struct TwitchGetStreamsResponse {
    data: Vec<TwitchStream>,
}

#[derive(Debug, Deserialize)]
struct TwitchStream {
    user_login: String,
    user_name: String,
    title: String,
    viewer_count: i64,
    started_at: String,
    #[serde(default)]
    game_name: String,
    thumbnail_url: String,
}

struct CachedAppAccessToken {
    value: String,
    expires_at: Instant,
}

impl TwitchApiService {
    pub fn new(client_id: String, client_secret: String) -> Self {
        Self {
            client_id,
            client_secret,
            token_url: DEFAULT_TWITCH_TOKEN_URL.to_string(),
            streams_url: DEFAULT_TWITCH_STREAMS_URL.to_string(),
            client: Client::new(),
            app_access_token: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn get_live_streams_by_user_logins(
        &self,
        user_logins: &[String],
    ) -> Result<Vec<TwitchLiveStream>, ApiError> {
        if user_logins.is_empty() {
            log::debug!("TwitchApiService: empty user_logins input, returning empty result");
            return Ok(Vec::new());
        }

        log::debug!(
            "TwitchApiService: fetching live streams for {} logins",
            user_logins.len()
        );

        let access_token = self.get_or_request_twitch_app_access_token().await?;
        let mut access_token = access_token;

        let mut streams = Vec::new();
        for (chunk_index, chunk) in user_logins.chunks(100).enumerate() {
            let mut query = vec![
                ("type".to_string(), "live".to_string()),
                ("first".to_string(), chunk.len().to_string()),
            ];
            for login in chunk {
                query.push(("user_login".to_string(), login.clone()));
            }

            log::debug!(
                "TwitchApiService: requesting helix/streams chunk={} size={}",
                chunk_index + 1,
                chunk.len()
            );
            let response = self
                .send_helix_get_with_token_retry_once(
                    &self.streams_url,
                    &mut access_token,
                    &query,
                    &format!("streams_chunk_{}", chunk_index + 1),
                )
                .await?;

            let status = response.status();
            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                log::error!(
                    "Twitch streams request failed: status={} body={}",
                    status,
                    body
                );
                return Err(ApiError::InternalError {
                    error: "Failed to load Twitch streams".to_string(),
                });
            }

            let payload = response
                .json::<TwitchGetStreamsResponse>()
                .await
                .map_err(|e| ApiError::InternalError {
                    error: format!("Failed to parse Twitch streams response: {}", e),
                })?;
            log::debug!(
                "TwitchApiService: helix/streams chunk={} returned {} live streams",
                chunk_index + 1,
                payload.data.len()
            );
            streams.extend(payload.data.into_iter().map(TwitchLiveStream::from));
        }

        streams.sort_by(|a, b| b.viewer_count.cmp(&a.viewer_count));
        log::info!(
            "TwitchApiService: resolved {} live streams from {} logins",
            streams.len(),
            user_logins.len()
        );
        Ok(streams)
    }

    async fn request_twitch_app_access_token(
        &self,
    ) -> Result<TwitchAppAccessTokenResponse, ApiError> {
        let client_id = self.client_id.trim();
        let client_secret = self.client_secret.trim();
        log::debug!("TwitchApiService: requesting new app access token");
        let response = self
            .client
            .post(&self.token_url)
            .query(&[
                ("client_id", client_id),
                ("client_secret", client_secret),
                ("grant_type", "client_credentials"),
            ])
            .send()
            .await
            .map_err(|e| ApiError::InternalError {
                error: format!("Failed to request Twitch token: {}", e),
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            log::error!(
                "Twitch token request failed: status={} body={}",
                status,
                body
            );
            return Err(ApiError::InternalError {
                error: "Failed to request Twitch token".to_string(),
            });
        }

        let payload = response
            .json::<TwitchAppAccessTokenResponse>()
            .await
            .map_err(|e| ApiError::InternalError {
                error: format!("Failed to parse Twitch token response: {}", e),
            })?;

        log::info!(
            "TwitchApiService: received app access token, expires_in={}s token_type={}",
            payload.expires_in,
            payload.token_type
        );

        Ok(payload)
    }

    async fn get_or_request_twitch_app_access_token(&self) -> Result<String, ApiError> {
        if let Some(token) = self.get_cached_app_access_token().await {
            log::debug!("TwitchApiService: using cached app access token");
            return Ok(token);
        }

        log::debug!("TwitchApiService: cached app access token miss");

        let token_response = self.request_twitch_app_access_token().await?;

        self.cache_app_access_token(
            token_response.access_token.clone(),
            token_response.expires_in,
        )
        .await;

        Ok(token_response.access_token)
    }

    async fn get_cached_app_access_token(&self) -> Option<String> {
        let mut state = self.app_access_token.lock().await;
        match state.as_ref() {
            Some(cached) if cached.expires_at > Instant::now() => Some(cached.value.clone()),
            Some(_) => {
                log::debug!("TwitchApiService: cached app access token expired, dropping");
                *state = None;
                None
            }
            None => None,
        }
    }

    async fn cache_app_access_token(&self, token: String, expires_in_seconds: u64) {
        let ttl = token_ttl_from_expires_in(expires_in_seconds);
        log::debug!(
            "TwitchApiService: caching app access token for {} seconds",
            ttl.as_secs()
        );
        let mut state = self.app_access_token.lock().await;
        *state = Some(CachedAppAccessToken {
            value: token,
            expires_at: Instant::now() + ttl,
        });
    }

    async fn invalidate_cached_app_access_token(&self) {
        log::debug!("TwitchApiService: invalidating cached app access token");
        let mut state = self.app_access_token.lock().await;
        *state = None;
    }

    async fn send_helix_get_with_token_retry_once(
        &self,
        endpoint: &str,
        access_token: &mut String,
        query: &[(String, String)],
        retry_context: &str,
    ) -> Result<reqwest::Response, ApiError> {
        let client_id = self.client_id.trim();
        let mut retried_with_new_token = false;

        loop {
            let response = self
                .client
                .get(endpoint)
                .header("Client-Id", client_id)
                .bearer_auth(access_token.as_str())
                .query(query)
                .send()
                .await
                .map_err(|e| ApiError::InternalError {
                    error: format!("Failed to call Twitch helix endpoint: {}", e),
                })?;

            if response.status() == StatusCode::UNAUTHORIZED && !retried_with_new_token {
                log::warn!(
                    "TwitchApiService: received 401 for {}, refreshing app token and retrying once",
                    retry_context
                );
                self.invalidate_cached_app_access_token().await;
                *access_token = self.get_or_request_twitch_app_access_token().await?;
                retried_with_new_token = true;
                continue;
            }

            return Ok(response);
        }
    }

    #[cfg(test)]
    fn with_urls(mut self, token_url: String, streams_url: String) -> Self {
        self.token_url = token_url;
        self.streams_url = streams_url;
        self
    }
}

fn token_ttl_from_expires_in(expires_in_seconds: u64) -> Duration {
    Duration::from_secs(expires_in_seconds).saturating_sub(TOKEN_EXPIRY_SAFETY_BUFFER)
}

impl From<TwitchStream> for TwitchLiveStream {
    fn from(value: TwitchStream) -> Self {
        Self {
            user_login: value.user_login,
            user_name: value.user_name,
            title: value.title,
            viewer_count: value.viewer_count,
            started_at: value.started_at,
            game_name: value.game_name,
            thumbnail_url: value.thumbnail_url,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{TwitchApiService, token_ttl_from_expires_in};
    use httpmock::Method::{GET, POST};
    use httpmock::MockServer;
    use std::time::Duration;

    #[test]
    fn token_ttl_subtracts_safety_buffer() {
        let ttl = token_ttl_from_expires_in(3600);
        assert_eq!(ttl, Duration::from_secs(3540));
    }

    #[test]
    fn token_ttl_is_zero_when_expires_in_below_buffer() {
        let ttl = token_ttl_from_expires_in(30);
        assert_eq!(ttl, Duration::from_secs(0));
    }

    #[tokio::test]
    async fn refreshes_token_on_401_and_retries_streams_once() {
        let server = MockServer::start_async().await;

        let token_refresh_mock = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path("/oauth2/token")
                    .query_param("grant_type", "client_credentials");
                then.status(200).json_body_obj(&serde_json::json!({
                    "access_token": "token_second",
                    "expires_in": 3600,
                    "token_type": "bearer"
                }));
            })
            .await;

        let streams_unauthorized_mock = server
            .mock_async(|when, then| {
                when.method(GET)
                    .path("/helix/streams")
                    .header("authorization", "Bearer token_first")
                    .query_param("type", "live")
                    .query_param("first", "1")
                    .query_param("user_login", "player1");
                then.status(401).body("unauthorized");
            })
            .await;

        let streams_success_mock = server
            .mock_async(|when, then| {
                when.method(GET)
                    .path("/helix/streams")
                    .header("authorization", "Bearer token_second")
                    .query_param("type", "live")
                    .query_param("first", "1")
                    .query_param("user_login", "player1");
                then.status(200).json_body_obj(&serde_json::json!({
                    "data": [{
                        "user_login": "player1",
                        "user_name": "Player One",
                        "title": "Ranked grind",
                        "viewer_count": 42,
                        "started_at": "2026-02-21T18:00:00Z",
                        "game_name": "Overwatch 2",
                        "thumbnail_url": "https://example.com/{width}x{height}.jpg"
                    }]
                }));
            })
            .await;

        let service = TwitchApiService::new("cid".to_string(), "secret".to_string()).with_urls(
            format!("{}/oauth2/token", server.base_url()),
            format!("{}/helix/streams", server.base_url()),
        );
        {
            let mut state = service.app_access_token.lock().await;
            *state = Some(super::CachedAppAccessToken {
                value: "token_first".to_string(),
                expires_at: std::time::Instant::now() + std::time::Duration::from_secs(300),
            });
        }

        let result = service
            .get_live_streams_by_user_logins(&["player1".to_string()])
            .await
            .expect("streams should load after token refresh");

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].user_login, "player1");

        assert_eq!(token_refresh_mock.calls_async().await, 1);
        assert_eq!(streams_unauthorized_mock.calls_async().await, 1);
        assert_eq!(streams_success_mock.calls_async().await, 1);
    }
}
