use crate::api::error::ApiError;
use crate::config::Config;
use crate::dal::registrations::RoleValue;
use actix_web::{HttpResponse, Responder, get, web};
use chrono::Utc;
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use super::models::{Tournament, TournamentShort};
use crate::dal::Dal;

#[get("")]
pub async fn get_tournaments(
    db: web::Data<Arc<Dal>>,
) -> actix_web::Result<impl Responder, ApiError> {
    let rows = db.tournaments.list_short().await.map_err(|e| {
        log::error!("Failed to load tournaments: {}", e);
        ApiError::InternalError {
            error: e.to_string(),
        }
    })?;

    let tournaments = rows
        .into_iter()
        .map(|row| TournamentShort {
            title: row.title,
            uri: row.uri,
            discipline: row.discipline,
            format: row.format,
            dates: row.dates,
            prize_pool: row.prize_pool,
            registration_count: row.registration_count,
        })
        .collect::<Vec<_>>();

    Ok(HttpResponse::Ok().json(json!(tournaments)))
}

#[get("/{sef_uri}")]
pub async fn get_tournament(
    db: web::Data<Arc<Dal>>,
    sef_uri: web::Path<String>,
) -> actix_web::Result<impl Responder, ApiError> {
    let sef_uri = sef_uri.into_inner();

    let row = db
        .tournaments
        .get_by_sef_title(&sef_uri)
        .await
        .map_err(|e| {
            log::error!("Failed to load tournament {}: {}", sef_uri, e);
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?
        .ok_or(ApiError::NotFound)?;

    let tournament = Tournament {
        id: row.sef_title,
        title: row.title,
        discipline: row.discipline,
        format: row.format,
        type_: row.config.type_,
        organizers: row.config.organizers,
        rules: row.config.rules,
        eligibility: row.config.eligibility,
        registration: row.config.registration,
        teams: row.config.teams,
        schedule: row.config.schedule,
        match_format: row.config.match_format,
        prize_pool: row.config.prize_pool,
        stream: row.config.stream,
        markdown: row.config.markdown,
    };

    Ok(HttpResponse::Ok().json(tournament))
}

#[derive(Serialize)]
struct PublicRegistrationSummary {
    battletag: String,
    primary_role: Option<RoleValue>,
}

#[derive(Clone, Serialize)]
struct LiveStreamSummary {
    user_login: String,
    user_name: String,
    title: String,
    viewer_count: i64,
    started_at: String,
    game_name: String,
    thumbnail_url: String,
}

struct CachedLiveStreams {
    value: Vec<LiveStreamSummary>,
    expires_at: Instant,
}

const LIVE_STREAMS_CACHE_TTL: Duration = Duration::from_secs(60 * 5);
const LIVE_STREAMS_EMPTY_CACHE_TTL: Duration = Duration::from_secs(30);

#[derive(Debug, Deserialize)]
struct TwitchAppAccessTokenResponse {
    access_token: String,
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

fn live_streams_cache() -> &'static Arc<Mutex<HashMap<String, CachedLiveStreams>>> {
    static CACHE: OnceLock<Arc<Mutex<HashMap<String, CachedLiveStreams>>>> = OnceLock::new();
    CACHE.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

#[get("/{sef_uri}/registrations")]
pub async fn get_tournament_registrations(
    db: web::Data<Arc<Dal>>,
    sef_uri: web::Path<String>,
) -> actix_web::Result<impl Responder, ApiError> {
    let sef_uri = sef_uri.into_inner();

    let tournament_id = db
        .tournaments
        .get_id_by_sef_title(&sef_uri)
        .await
        .map_err(|e| {
            log::error!("Failed to load tournament id for {}: {}", sef_uri, e);
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?
        .ok_or(ApiError::NotFound)?;

    let rows = db
        .registrations
        .list_public_summaries(tournament_id)
        .await
        .map_err(|e| {
            log::error!(
                "Failed to load public registrations for tournament {}: {}",
                sef_uri,
                e
            );
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?;

    let response = rows
        .into_iter()
        .map(|row| PublicRegistrationSummary {
            battletag: row.battletag,
            primary_role: row.primary_role,
        })
        .collect::<Vec<_>>();

    Ok(HttpResponse::Ok().json(json!(response)))
}

#[get("/{sef_uri}/live-streams")]
pub async fn get_tournament_live_streams(
    db: web::Data<Arc<Dal>>,
    config: web::Data<Arc<Config>>,
    sef_uri: web::Path<String>,
) -> actix_web::Result<impl Responder, ApiError> {
    let sef_uri = sef_uri.into_inner();

    let tournament = db
        .tournaments
        .get_id_and_dates_by_sef_title(&sef_uri)
        .await
        .map_err(|e| {
            log::error!("Failed to load tournament meta for {}: {}", sef_uri, e);
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?
        .ok_or(ApiError::NotFound)?;

    if !is_today_tournament_day(&tournament.dates) {
        return Ok(HttpResponse::Ok().json(Vec::<LiveStreamSummary>::new()));
    }

    let tournament_id = tournament.id;

    let cache_key = sef_uri.clone();
    if let Some(cached) = get_cached_live_streams(&cache_key).await {
        return Ok(HttpResponse::Ok().json(cached));
    }

    let rows = db
        .registrations
        .list_public_twitch_logins(tournament_id)
        .await
        .map_err(|e| {
            log::error!(
                "Failed to load twitch logins for tournament {}: {}",
                sef_uri,
                e
            );
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?;

    let mut seen = HashSet::new();
    let twitch_logins = rows
        .into_iter()
        .map(|row| row.twitch.trim().to_lowercase())
        .filter(|login| !login.is_empty())
        .filter(|login| seen.insert(login.clone()))
        .collect::<Vec<_>>();

    if twitch_logins.is_empty() {
        let empty = Vec::<LiveStreamSummary>::new();
        set_cached_live_streams(cache_key, &empty, LIVE_STREAMS_EMPTY_CACHE_TTL).await;
        return Ok(HttpResponse::Ok().json(empty));
    }

    let twitch_client_id = config
        .twitch
        .client_id
        .as_deref()
        .map(str::trim)
        .unwrap_or_default();
    let twitch_client_secret = config
        .twitch
        .client_secret
        .as_deref()
        .map(str::trim)
        .unwrap_or_default();

    if twitch_client_id.is_empty() || twitch_client_secret.is_empty() {
        log::warn!(
            "TWITCH_CLIENT_ID/TWITCH_CLIENT_SECRET are not configured. Live streams list is empty."
        );
        let empty = Vec::<LiveStreamSummary>::new();
        set_cached_live_streams(cache_key, &empty, LIVE_STREAMS_EMPTY_CACHE_TTL).await;
        return Ok(HttpResponse::Ok().json(empty));
    }

    let client = Client::new();
    let access_token =
        request_twitch_app_access_token(&client, twitch_client_id, twitch_client_secret).await?;

    let mut streams = Vec::new();

    for chunk in twitch_logins.chunks(100) {
        let mut query = vec![
            ("type".to_string(), "live".to_string()),
            ("first".to_string(), chunk.len().to_string()),
        ];
        for login in chunk {
            query.push(("user_login".to_string(), login.clone()));
        }

        let response = client
            .get("https://api.twitch.tv/helix/streams")
            .header("Client-Id", twitch_client_id)
            .bearer_auth(&access_token)
            .query(&query)
            .send()
            .await
            .map_err(|e| ApiError::InternalError {
                error: format!("Failed to load Twitch streams: {}", e),
            })?;

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
        streams.extend(payload.data);
    }

    streams.sort_by(|a, b| b.viewer_count.cmp(&a.viewer_count));

    let response = streams
        .into_iter()
        .map(|stream| LiveStreamSummary {
            user_login: stream.user_login,
            user_name: stream.user_name,
            title: stream.title,
            viewer_count: stream.viewer_count,
            started_at: stream.started_at,
            game_name: stream.game_name,
            thumbnail_url: stream.thumbnail_url,
        })
        .collect::<Vec<_>>();

    let ttl = if response.is_empty() {
        LIVE_STREAMS_EMPTY_CACHE_TTL
    } else {
        LIVE_STREAMS_CACHE_TTL
    };
    set_cached_live_streams(cache_key, &response, ttl).await;

    Ok(HttpResponse::Ok().json(response))
}

async fn request_twitch_app_access_token(
    client: &Client,
    client_id: &str,
    client_secret: &str,
) -> Result<String, ApiError> {
    let response = client
        .post("https://id.twitch.tv/oauth2/token")
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

    Ok(payload.access_token)
}

async fn get_cached_live_streams(cache_key: &str) -> Option<Vec<LiveStreamSummary>> {
    let cache = live_streams_cache();
    let mut state = cache.lock().await;
    match state.get(cache_key) {
        Some(cached) if cached.expires_at > Instant::now() => Some(cached.value.clone()),
        Some(_) => {
            state.remove(cache_key);
            None
        }
        None => None,
    }
}

async fn set_cached_live_streams(cache_key: String, streams: &[LiveStreamSummary], ttl: Duration) {
    let cache = live_streams_cache();
    let mut state = cache.lock().await;
    state.insert(
        cache_key,
        CachedLiveStreams {
            value: streams.to_vec(),
            expires_at: Instant::now() + ttl,
        },
    );
}

fn is_today_tournament_day(dates: &[String]) -> bool {
    let today = Utc::now().date_naive().format("%Y-%m-%d").to_string();
    dates.iter().any(|date| date.trim() == today)
}
