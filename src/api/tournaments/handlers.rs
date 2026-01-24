use crate::api::error::ApiError;
use actix_web::{HttpResponse, Responder, get, web};
use serde_json::json;
use std::sync::Arc;

use super::models::{Tournament, TournamentShort};
use crate::dal::Dal;

#[get("")]
pub async fn get_tournaments(
    db: web::Data<Arc<Dal>>,
) -> actix_web::Result<impl Responder, ApiError> {
    let rows = db.tournaments.list_short().await.map_err(|e| {
        log::error!("Failed to load tournaments: {}", e);
        ApiError::InternalError { error: e.to_string() }
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
        })
        .collect::<Vec<_>>();

    Ok(HttpResponse::Ok().json(json!(tournaments)))
}

#[get("/{id}")]
pub async fn get_tournament(
    db: web::Data<Arc<Dal>>,
    id: web::Path<String>,
) -> actix_web::Result<impl Responder, ApiError> {
    let id = id.into_inner();
    
    // Basic path traversal protection
    if id.contains("..") || id.contains('/') || id.contains('\\') {
        return Err(ApiError::BadRequest {
            error: "Invalid ID".to_string(),
            details: "ID contains invalid characters".to_string(),
        });
    }

    let row = db
        .tournaments
        .get_by_sef_title(&id)
        .await
        .map_err(|e| {
            log::error!("Failed to load tournament {}: {}", id, e);
            ApiError::InternalError { error: e.to_string() }
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
