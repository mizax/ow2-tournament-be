use crate::api::ApiError;
use crate::dal::Dal;
use crate::services::TtlCache;
use chrono::Utc;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use super::twitch_api_service::{TwitchApiService, TwitchLiveStream};

const LIVE_STREAMS_CACHE_TTL: Duration = Duration::from_secs(60 * 5);
const LIVE_STREAMS_EMPTY_CACHE_TTL: Duration = Duration::from_secs(30);

#[derive(Clone, Serialize)]
pub struct LiveStreamSummary {
    pub user_login: String,
    pub user_name: String,
    pub title: String,
    pub viewer_count: i64,
    pub started_at: String,
    pub game_name: String,
    pub thumbnail_url: String,
}

pub struct TournamentLiveStreamsService {
    twitch_api_service: Arc<TwitchApiService>,
    cache: TtlCache<String, Option<LiveStreamSummary>>,
}

impl TournamentLiveStreamsService {
    pub fn new(twitch_api_service: Arc<TwitchApiService>) -> Self {
        Self {
            twitch_api_service,
            cache: TtlCache::new(),
        }
    }

    pub async fn get_tournament_live_streams(
        &self,
        db: &Dal,
        sef_uri: &str,
    ) -> Result<Vec<LiveStreamSummary>, ApiError> {
        log::debug!(
            "TournamentLiveStreamsService: resolving live streams for tournament={}",
            sef_uri
        );
        let tournament = db
            .tournaments
            .get_id_and_dates_by_sef_title(sef_uri)
            .await
            .map_err(|e| {
                log::error!("Failed to load tournament meta for {}: {}", sef_uri, e);
                ApiError::InternalError {
                    error: e.to_string(),
                }
            })?
            .ok_or(ApiError::NotFound)?;

        if !is_today_tournament_day(&tournament.dates) {
            log::debug!(
                "TournamentLiveStreamsService: tournament={} is not active today, returning empty result",
                sef_uri
            );
            return Ok(Vec::new());
        }

        let rows = db
            .registrations
            .list_public_twitch_logins(tournament.id)
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

        log::debug!(
            "TournamentLiveStreamsService: loaded {} twitch logins from registrations for tournament={}",
            rows.len(),
            sef_uri
        );

        let mut seen = HashSet::new();
        let twitch_logins = rows
            .into_iter()
            .map(|row| row.twitch.trim().to_lowercase())
            .filter(|login| !login.is_empty())
            .filter(|login| seen.insert(login.clone()))
            .collect::<Vec<_>>();

        log::debug!(
            "TournamentLiveStreamsService: {} unique non-empty twitch logins for tournament={}",
            twitch_logins.len(),
            sef_uri
        );

        if twitch_logins.is_empty() {
            log::debug!(
                "TournamentLiveStreamsService: no twitch logins for tournament={}",
                sef_uri,
            );
            return Ok(Vec::new());
        }

        let mut response = Vec::new();
        let mut missing_logins = Vec::new();
        let mut cached_live_hits = 0_usize;
        let mut cached_offline_hits = 0_usize;
        for login in &twitch_logins {
            match self.cache.get(login).await {
                Some(Some(stream)) => {
                    response.push(stream);
                    cached_live_hits += 1;
                }
                Some(None) => {
                    cached_offline_hits += 1;
                }
                None => missing_logins.push(login.clone()),
            }
        }

        log::debug!(
            "TournamentLiveStreamsService: per-login cache stats tournament={} live_hits={} offline_hits={} misses={}",
            sef_uri,
            cached_live_hits,
            cached_offline_hits,
            missing_logins.len()
        );

        if !missing_logins.is_empty() {
            let fetched_map: HashMap<String, LiveStreamSummary> = self
                .twitch_api_service
                .get_live_streams_by_user_logins(&missing_logins)
                .await?
                .into_iter()
                .map(LiveStreamSummary::from)
                .map(|stream| (stream.user_login.to_lowercase(), stream))
                .collect();

            let mut fetched_live_count = 0_usize;
            let mut fetched_offline_count = 0_usize;
            for login in missing_logins {
                if let Some(stream) = fetched_map.get(&login).cloned() {
                    self.cache
                        .set(login.clone(), Some(stream.clone()), LIVE_STREAMS_CACHE_TTL)
                        .await;
                    response.push(stream);
                    fetched_live_count += 1;
                } else {
                    self.cache
                        .set(login, None, LIVE_STREAMS_EMPTY_CACHE_TTL)
                        .await;
                    fetched_offline_count += 1;
                }
            }

            log::debug!(
                "TournamentLiveStreamsService: fetched+cached tournament={} fetched_live={} fetched_offline={}",
                sef_uri,
                fetched_live_count,
                fetched_offline_count
            );
        }

        response.sort_by(|a, b| b.viewer_count.cmp(&a.viewer_count));

        log::info!(
            "TournamentLiveStreamsService: tournament={} live_streams={}",
            sef_uri,
            response.len(),
        );

        Ok(response)
    }
}

fn is_today_tournament_day(dates: &[String]) -> bool {
    let today = Utc::now().date_naive().format("%Y-%m-%d").to_string();
    dates.iter().any(|date| date.trim() == today)
}

impl From<TwitchLiveStream> for LiveStreamSummary {
    fn from(value: TwitchLiveStream) -> Self {
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
