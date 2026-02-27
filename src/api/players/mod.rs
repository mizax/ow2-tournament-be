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

#[derive(sqlx::FromRow)]
struct PlayerProfileRow {
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

    let row = sqlx::query_as::<_, PlayerProfileRow>(
        r#"
        SELECT p.id, p.nickname, p.role, p.registration_id,
               t.name as team_name,
               tour.title as tournament_title, tour.sef_title as tournament_sef,
               d.name as division_name,
               ub.battletag
        FROM players p
        JOIN teams t ON p.team_id = t.id
        JOIN tournaments tour ON t.tournament_id = tour.id
        LEFT JOIN divisions d ON p.division_id = d.id
        LEFT JOIN registrations r ON p.registration_id = r.id
        LEFT JOIN user_battletags ub ON r.user_battletag_id = ub.id
        WHERE p.id = ?1 AND p.deleted_at IS NULL
        "#,
    )
    .bind(player_id)
    .fetch_optional(&db.db_pool)
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

#[derive(sqlx::FromRow)]
struct PlayerMatchRow {
    match_id: i64,
    home_score: Option<i64>,
    away_score: Option<i64>,
    home_team: String,
    away_team: String,
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

    let rows = sqlx::query_as::<_, PlayerMatchRow>(
        r#"
        SELECT m.id as match_id, m.home_score, m.away_score,
               ht.name as home_team, at.name as away_team,
               tour.title as tournament_title,
               COUNT(DISTINCT mps.match_map_id) as maps_played,
               SUM(mps.eliminations) as kills,
               SUM(mps.deaths) as deaths,
               SUM(mps.all_damage) as damage,
               SUM(mps.healing_dealt) as healing,
               SUM(mps.hero_time_played) as time_played
        FROM match_player_statistics mps
        JOIN match_maps mm ON mps.match_map_id = mm.id
        JOIN matches m ON mm.match_id = m.id
        JOIN teams ht ON m.home_team_id = ht.id
        JOIN teams at ON m.away_team_id = at.id
        JOIN tournaments tour ON m.tournament_id = tour.id
        WHERE mps.player_id = ?1
        GROUP BY m.id
        ORDER BY m.id DESC
        "#,
    )
    .bind(player_id)
    .fetch_all(&db.db_pool)
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
