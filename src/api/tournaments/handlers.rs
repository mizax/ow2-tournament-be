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
