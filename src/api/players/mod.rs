use actix_web::{HttpResponse, Responder, get, web};
use serde::Serialize;
use std::sync::Arc;

use crate::api::error::ApiError;
use crate::dal::Dal;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/players")
            .service(get_player)
            .service(get_player_matches),
    );
}

#[derive(Serialize)]
struct PlayerProfile {
    id: i64,
    nickname: String,
    role: Option<String>,
    registration_id: Option<i64>,
    battletag: Option<String>,
    team_name: String,
    tournament_title: String,
    tournament_sef: String,
    division_name: Option<String>,
}

#[get("/{player_id}")]
async fn get_player(
    db: web::Data<Arc<Dal>>,
    player_id: web::Path<i64>,
) -> actix_web::Result<impl Responder, ApiError> {
    let player_id = player_id.into_inner();

    let row = db
        .players
        .get_public_profile(player_id)
        .await
        .map_err(|e| {
            log::error!("Failed to load player {}: {}", player_id, e);
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?
        .ok_or(ApiError::NotFound)?;

    let profile = PlayerProfile {
        id: row.id,
        nickname: row.nickname,
        role: row.role,
        registration_id: row.registration_id,
        battletag: row.battletag,
        team_name: row.team_name,
        tournament_title: row.tournament_title,
        tournament_sef: row.tournament_sef,
        division_name: row.division_name,
    };

    Ok(HttpResponse::Ok().json(profile))
}

#[derive(Serialize)]
struct PlayerMatchSummary {
    match_id: i64,
    home_team: String,
    away_team: String,
    home_score: Option<i64>,
    away_score: Option<i64>,
    tournament_title: String,
    maps_played: i64,
    kills: Option<i64>,
    deaths: Option<i64>,
    damage: Option<f64>,
    healing: Option<f64>,
    time_played: Option<f64>,
}

#[get("/{player_id}/matches")]
async fn get_player_matches(
    db: web::Data<Arc<Dal>>,
    player_id: web::Path<i64>,
) -> actix_web::Result<impl Responder, ApiError> {
    let player_id = player_id.into_inner();

    let rows = db
        .players
        .list_public_match_summaries(player_id)
        .await
        .map_err(|e| {
            log::error!("Failed to load player matches {}: {}", player_id, e);
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?;

    let summaries: Vec<PlayerMatchSummary> = rows
        .into_iter()
        .map(|row| PlayerMatchSummary {
            match_id: row.match_id,
            home_team: row.home_team,
            away_team: row.away_team,
            home_score: row.home_score,
            away_score: row.away_score,
            tournament_title: row.tournament_title,
            maps_played: row.maps_played,
            kills: row.kills,
            deaths: row.deaths,
            damage: row.damage,
            healing: row.healing,
            time_played: row.time_played,
        })
        .collect();

    Ok(HttpResponse::Ok().json(summaries))
}
