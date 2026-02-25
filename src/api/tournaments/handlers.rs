use crate::api::error::ApiError;
use crate::dal::registrations::RoleValue;
use actix_web::{HttpResponse, Responder, get, web};
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;

use super::models::{Tournament, TournamentShort};
use crate::dal::Dal;
use crate::services::TournamentLiveStreamsService;

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
        status: row.config.status,
        results: row.config.results,
        media: row.config.media,
        markdown: row.config.markdown,
    };

    Ok(HttpResponse::Ok().json(tournament))
}

#[derive(Serialize)]
struct PublicRegistrationSummary {
    battletag: String,
    primary_role: Option<RoleValue>,
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

#[derive(Serialize)]
struct MatchSummary {
    id: i64,
    home_team: String,
    away_team: String,
    home_score: Option<i64>,
    away_score: Option<i64>,
    maps: Vec<TournamentMapScore>,
}

#[derive(Serialize)]
struct TournamentMapScore {
    map_order: i64,
    map_name: Option<String>,
    mode_name: Option<String>,
    home_score: i64,
    away_score: i64,
}

#[derive(sqlx::FromRow)]
struct TournamentMatchFlatRow {
    id: i64,
    home_score: Option<i64>,
    away_score: Option<i64>,
    home_team: String,
    away_team: String,
    map_id: Option<i64>,
    map_order: Option<i64>,
    map_home: Option<i64>,
    map_away: Option<i64>,
    map_name: Option<String>,
    mode_name: Option<String>,
}

#[get("/{sef_uri}/matches")]
pub async fn get_tournament_matches(
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

    let rows = sqlx::query_as::<_, TournamentMatchFlatRow>(
        r#"
        SELECT m.id, m.home_score, m.away_score,
               ht.name as home_team, at.name as away_team,
               mm.id as map_id, mm.map_order, mm.home_score as map_home, mm.away_score as map_away,
               maps.name as map_name, modes.name as mode_name
        FROM matches m
        JOIN teams ht ON m.home_team_id = ht.id
        JOIN teams at ON m.away_team_id = at.id
        LEFT JOIN match_maps mm ON mm.match_id = m.id
        LEFT JOIN game_maps gm ON mm.game_map_id = gm.id
        LEFT JOIN maps ON gm.map_id = maps.id
        LEFT JOIN modes ON gm.mode_id = modes.id
        WHERE m.tournament_id = ?1 AND m.deleted_at IS NULL
        ORDER BY m.id, mm.map_order
        "#,
    )
    .bind(tournament_id)
    .fetch_all(&db.db_pool)
    .await
    .map_err(|e| {
        log::error!("Failed to load matches for tournament {}: {}", sef_uri, e);
        ApiError::InternalError {
            error: e.to_string(),
        }
    })?;

    // Group rows by match id, preserving insertion order
    let mut match_ids: Vec<i64> = Vec::new();
    let mut matches_map: std::collections::HashMap<
        i64,
        (
            String,
            String,
            Option<i64>,
            Option<i64>,
            Vec<TournamentMapScore>,
        ),
    > = std::collections::HashMap::new();

    for row in rows {
        if !matches_map.contains_key(&row.id) {
            match_ids.push(row.id);
            matches_map.insert(
                row.id,
                (
                    row.home_team.clone(),
                    row.away_team.clone(),
                    row.home_score,
                    row.away_score,
                    Vec::new(),
                ),
            );
        }
        if let Some(map_order) = row.map_order {
            if let Some(entry) = matches_map.get_mut(&row.id) {
                entry.4.push(TournamentMapScore {
                    map_order,
                    map_name: row.map_name.clone(),
                    mode_name: row.mode_name.clone(),
                    home_score: row.map_home.unwrap_or(0),
                    away_score: row.map_away.unwrap_or(0),
                });
            }
        }
    }

    let summaries: Vec<MatchSummary> =
        match_ids
            .into_iter()
            .filter_map(|id| {
                matches_map.remove(&id).map(
                    |(home_team, away_team, home_score, away_score, maps)| MatchSummary {
                        id,
                        home_team,
                        away_team,
                        home_score,
                        away_score,
                        maps,
                    },
                )
            })
            .collect();

    Ok(HttpResponse::Ok().json(serde_json::json!(summaries)))
}

#[get("/{sef_uri}/live-streams")]
pub async fn get_tournament_live_streams(
    db: web::Data<Arc<Dal>>,
    twitch_live_streams_service: Option<web::Data<TournamentLiveStreamsService>>,
    sef_uri: web::Path<String>,
) -> actix_web::Result<impl Responder, ApiError> {
    let sef_uri = sef_uri.into_inner();
    let Some(twitch_live_streams_service) = twitch_live_streams_service else {
        log::debug!(
            "Tournament live streams service is not configured, returning empty result for tournament={}",
            sef_uri
        );
        return Ok(HttpResponse::Ok().json(Vec::<crate::services::LiveStreamSummary>::new()));
    };
    let response = twitch_live_streams_service
        .get_tournament_live_streams(db.get_ref().as_ref(), &sef_uri)
        .await?;
    Ok(HttpResponse::Ok().json(response))
}
