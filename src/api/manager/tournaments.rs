use actix_web::{HttpResponse, Responder, get, web};
use chrono::NaiveDateTime;
use serde::Serialize;
use std::sync::Arc;

use crate::api::auth::jwt::AuthenticatedUser;
use crate::api::error::ApiError;
use crate::dal::{Dal, ManagedTournamentRow, MatchWithTeamsRow};

#[derive(Debug, Serialize)]
struct ManagedTournamentResponse {
    pub id: i64,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sef: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_at: Option<NaiveDateTime>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registration_count: Option<i64>,
}

#[derive(Debug, Serialize)]
struct MatchResponse {
    pub id: i64,
    pub home_team_id: i64,
    pub home_team_name: String,
    pub away_team_id: i64,
    pub away_team_name: String,
    pub home_score: i64,
    pub away_score: i64,
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/tournaments")
            .service(list_managed_tournaments)
            .service(list_tournament_matches),
    );
}

#[get("")]
pub async fn list_managed_tournaments(
    user: AuthenticatedUser,
    db: web::Data<Arc<Dal>>,
) -> actix_web::Result<impl Responder, ApiError> {
    let user_id = user.id.parse::<i64>().map_err(|_| ApiError::BadRequest {
        error: "Invalid user id".to_string(),
        details: "User id from auth token is not a number.".to_string(),
    })?;

    let tournaments = db
        .tournament_managers
        .list_managed_tournaments(user_id)
        .await
        .map_err(|e| {
            log::error!(
                "Failed to load managed tournaments for user {}: {}",
                user_id,
                e
            );
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?;

    let response = tournaments
        .into_iter()
        .map(map_managed_tournament)
        .collect::<Vec<_>>();

    Ok(HttpResponse::Ok().json(response))
}

#[get("/{tournament_id}/matches")]
pub async fn list_tournament_matches(
    path: web::Path<i64>,
    _user: AuthenticatedUser,
    db: web::Data<Arc<Dal>>,
) -> actix_web::Result<impl Responder, ApiError> {
    let tournament_id = path.into_inner();

    let matches = db
        .matches
        .list_by_tournament(tournament_id)
        .await
        .map_err(|e| {
            log::error!(
                "Failed to load matches for tournament {}: {}",
                tournament_id,
                e
            );
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?;

    let response = matches.into_iter().map(map_match).collect::<Vec<_>>();

    Ok(HttpResponse::Ok().json(response))
}

fn map_match(row: MatchWithTeamsRow) -> MatchResponse {
    MatchResponse {
        id: row.id,
        home_team_id: row.home_team_id,
        home_team_name: row.home_team_name,
        away_team_id: row.away_team_id,
        away_team_name: row.away_team_name,
        home_score: row.home_score,
        away_score: row.away_score,
    }
}

fn map_managed_tournament(row: ManagedTournamentRow) -> ManagedTournamentResponse {
    let sef = match row.sef_title.trim() {
        "" => None,
        value => Some(value.to_string()),
    };

    ManagedTournamentResponse {
        id: row.id,
        title: row.title,
        sef,
        start_at: row.started_at,
        registration_count: Some(row.registration_count),
    }
}
