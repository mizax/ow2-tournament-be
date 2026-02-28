use actix_web::{HttpResponse, Responder, get, put, web};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::Arc;

use crate::api::auth::jwt::{AuthenticatedUser, UserRole};
use crate::api::error::ApiError;
use crate::dal::{
    Dal, ManagedTournamentRow, MatchWithTeamsRow, TournamentDetailsData, UpdateTournamentData,
};
use crate::shared_models::tournaments::models::*;

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

#[derive(Debug, Deserialize)]
pub struct UpdateTournamentRequest {
    pub title: String,
    pub sef_title: String,
    pub discipline: String,
    pub format: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub organizers: Option<Vec<Organizer>>,
    pub rules: Option<Rules>,
    pub eligibility: Option<Eligibility>,
    pub registration: Option<Registration>,
    pub teams: Option<Teams>,
    pub schedule: Vec<ScheduleItem>,
    pub match_format: Option<MatchFormat>,
    pub prize_pool: PrizePool,
    pub stream: Option<Stream>,
    pub status: Option<TournamentStatus>,
    pub results: Option<TournamentResults>,
    pub media: Option<TournamentMedia>,
    pub markdown: Option<Markdown>,
}

#[derive(Debug, Serialize)]
pub struct TournamentFullResponse {
    pub id: i64,
    pub title: String,
    pub sef_title: String,
    pub discipline: String,
    pub format: String,
    #[serde(rename = "type")]
    pub type_: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organizers: Option<Vec<Organizer>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rules: Option<Rules>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eligibility: Option<Eligibility>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub registration: Option<Registration>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub teams: Option<Teams>,
    pub schedule: Vec<ScheduleItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_format: Option<MatchFormat>,
    pub prize_pool: PrizePool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<Stream>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<TournamentStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<TournamentResults>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media: Option<TournamentMedia>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markdown: Option<Markdown>,
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/tournaments")
            .service(list_managed_tournaments)
            .service(get_managed_tournament)
            .service(update_tournament)
            .service(list_tournament_matches),
    );
}

#[get("")]
pub async fn list_managed_tournaments(
    user: AuthenticatedUser,
    db: web::Data<Arc<Dal>>,
) -> actix_web::Result<impl Responder, ApiError> {
    let user_id = parse_user_id(&user)?;

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

#[get("/{tournament_id}")]
pub async fn get_managed_tournament(
    user: AuthenticatedUser,
    path: web::Path<i64>,
    db: web::Data<Arc<Dal>>,
) -> actix_web::Result<impl Responder, ApiError> {
    let tournament_id = path.into_inner();
    let user_id = parse_user_id(&user)?;

    ensure_can_manage_tournament(&db, &user, tournament_id, user_id).await?;

    let tournament = db
        .tournaments
        .get_by_id(tournament_id)
        .await
        .map_err(map_tournament_repo_error)?
        .ok_or(ApiError::NotFound)?;

    Ok(HttpResponse::Ok().json(map_tournament_full_response(tournament)))
}

#[put("/{tournament_id}")]
pub async fn update_tournament(
    user: AuthenticatedUser,
    path: web::Path<i64>,
    payload: web::Json<UpdateTournamentRequest>,
    db: web::Data<Arc<Dal>>,
) -> actix_web::Result<impl Responder, ApiError> {
    let tournament_id = path.into_inner();
    let user_id = parse_user_id(&user)?;
    let request = payload.into_inner();

    ensure_can_manage_tournament(&db, &user, tournament_id, user_id).await?;

    let existing = db
        .tournaments
        .get_by_id(tournament_id)
        .await
        .map_err(map_tournament_repo_error)?
        .ok_or(ApiError::NotFound)?;

    if existing.sef_title != request.sef_title {
        let existing_id = db
            .tournaments
            .get_id_by_sef_title(&request.sef_title)
            .await
            .map_err(|e| {
                log::error!(
                    "Failed to check sef_title conflict for tournament {} and sef_title {}: {}",
                    tournament_id,
                    request.sef_title,
                    e
                );
                ApiError::InternalError {
                    error: e.to_string(),
                }
            })?;

        if existing_id.is_some_and(|id| id != tournament_id) {
            return Err(ApiError::Conflict {
                error: "sef_title_taken".to_string(),
                details: "Tournament with this sef_title already exists".to_string(),
            });
        }
    }

    let UpdateTournamentRequest {
        title,
        sef_title,
        discipline,
        format,
        type_,
        organizers,
        rules,
        eligibility,
        registration,
        teams,
        schedule,
        match_format,
        prize_pool,
        stream,
        status,
        results,
        media,
        markdown,
    } = request;

    let unique_dates: BTreeSet<String> = schedule
        .iter()
        .map(|item| item.date.format("%Y-%m-%d").to_string())
        .collect();
    let dates_json = serde_json::to_string(&unique_dates.into_iter().collect::<Vec<String>>())
        .map_err(|e| {
            log::error!(
                "Failed to serialize tournament dates for tournament {}: {}",
                tournament_id,
                e
            );
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?;

    let prize_pool_total_amount: f64 = prize_pool
        .places
        .as_ref()
        .map(|places| places.iter().map(|place| place.amount).sum())
        .unwrap_or_default();

    let prize_pool_currency = prize_pool.currency.clone().unwrap_or_default();
    let config = TournamentConfig {
        type_,
        organizers,
        rules,
        eligibility,
        registration,
        teams,
        schedule,
        match_format,
        prize_pool,
        stream,
        status,
        results,
        media,
        markdown,
    };

    let configuration_json = serde_json::to_string(&config).map_err(|e| {
        log::error!(
            "Failed to serialize tournament config for tournament {}: {}",
            tournament_id,
            e
        );
        ApiError::InternalError {
            error: e.to_string(),
        }
    })?;

    db.tournaments
        .update(
            tournament_id,
            UpdateTournamentData {
                title,
                sef_title,
                discipline,
                format,
                dates_json,
                prize_pool_total_amount,
                prize_pool_currency,
                configuration_json,
            },
        )
        .await
        .map_err(map_tournament_repo_error)?;

    let updated = db
        .tournaments
        .get_by_id(tournament_id)
        .await
        .map_err(map_tournament_repo_error)?
        .ok_or(ApiError::NotFound)?;

    Ok(HttpResponse::Ok().json(map_tournament_full_response(updated)))
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

fn map_tournament_full_response(row: TournamentDetailsData) -> TournamentFullResponse {
    TournamentFullResponse {
        id: row.id,
        title: row.title,
        sef_title: row.sef_title,
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
    }
}

fn map_tournament_repo_error(error: crate::dal::TournamentsRepoError) -> ApiError {
    if let crate::dal::TournamentsRepoError::Db(sqlx::Error::Database(db_error)) = &error {
        let is_sef_title_unique_error = db_error.is_unique_violation()
            && db_error
                .message()
                .contains("UNIQUE constraint failed: tournaments.sef_title");
        if is_sef_title_unique_error {
            return ApiError::Conflict {
                error: "sef_title_taken".to_string(),
                details: "Tournament with this sef_title already exists".to_string(),
            };
        }
    }

    log::error!("Tournament repository error: {}", error);
    ApiError::InternalError {
        error: error.to_string(),
    }
}

fn parse_user_id(user: &AuthenticatedUser) -> Result<i64, ApiError> {
    user.id.parse::<i64>().map_err(|_| ApiError::BadRequest {
        error: "Invalid user id".to_string(),
        details: "User id from auth token is not a number.".to_string(),
    })
}

async fn ensure_can_manage_tournament(
    db: &Dal,
    user: &AuthenticatedUser,
    tournament_id: i64,
    user_id: i64,
) -> Result<(), ApiError> {
    if user.roles.contains(&UserRole::Admin) {
        return Ok(());
    }

    let is_manager = db
        .tournament_managers
        .user_is_manager_for_tournament(tournament_id, user_id)
        .await
        .map_err(|e| {
            log::error!(
                "Failed to check manager permissions for user {} in tournament {}: {}",
                user_id,
                tournament_id,
                e
            );
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?;

    if is_manager {
        Ok(())
    } else {
        Err(ApiError::Forbidden)
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared_models::tournaments::models::{PrizePool, TournamentConfig};

    #[test]
    fn tournament_full_response_omits_null_optional_fields() {
        let details = TournamentDetailsData {
            id: 1,
            title: "Test".to_string(),
            sef_title: "test".to_string(),
            discipline: "OW2".to_string(),
            format: "Online".to_string(),
            config: TournamentConfig {
                type_: "Online".to_string(),
                organizers: None,
                rules: None,
                eligibility: None,
                registration: None,
                teams: None,
                schedule: vec![],
                match_format: None,
                prize_pool: PrizePool {
                    currency: None,
                    places: None,
                },
                stream: None,
                status: None,
                results: Some(TournamentResults {
                    placements: None,
                    mvp: None,
                    summary: Some("ok".to_string()),
                }),
                media: None,
                markdown: None,
            },
        };

        let response = map_tournament_full_response(details);
        let json = serde_json::to_value(response).expect("serialize response");

        assert!(
            !json["results"]
                .as_object()
                .expect("results object")
                .contains_key("mvp")
        );
    }
}
