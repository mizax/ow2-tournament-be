use crate::api::auth::jwt::AuthenticatedUser;
use crate::api::error::ApiError;
use crate::dal::Dal;
use crate::logs_parser::{events_parser, saver};
use actix_web::{HttpResponse, Responder, post, web};
use log::error;
use std::sync::Arc;

#[post("/load/{match_id}/{log_name}")]
async fn load_log(
    path: web::Path<(i64, String)>,
    body: web::Bytes,
    db: web::Data<Arc<Dal>>,
    user: AuthenticatedUser,
) -> Result<impl Responder, ApiError> {
    let (match_id, log_name) = path.into_inner();

    let user_id = user.id.parse::<i64>().map_err(|_| ApiError::BadRequest {
        error: "Invalid user id".to_string(),
        details: "User id from auth token is not a number.".to_string(),
    })?;

    let match_row = db
        .matches
        .get_by_id(match_id)
        .await
        .map_err(|e| ApiError::InternalError {
            error: e.to_string(),
        })?
        .ok_or(ApiError::NotFound)?;

    if !user.is_admin() {
        let is_manager = db
            .tournament_managers
            .user_is_manager_for_tournament(match_row.tournament_id, user_id)
            .await
            .map_err(|e| {
                error!(
                    "Failed to check manager permissions for user {} in tournament {}: {}",
                    user_id, match_row.tournament_id, e
                );
                ApiError::InternalError {
                    error: e.to_string(),
                }
            })?;

        if !is_manager {
            return Err(ApiError::Forbidden);
        }
    }

    let csv = String::from_utf8(body.to_vec()).map_err(|e| ApiError::BadRequest {
        error: "logs.invalid_encoding".to_string(),
        details: e.to_string(),
    })?;

    let events = events_parser::parse_csv(&csv).map_err(|e| ApiError::BadRequest {
        error: "logs.parse_error".to_string(),
        details: e.to_string(),
    })?;

    saver::save_events(&db, &match_row, &log_name, &events)
        .await
        .map_err(|e| match e {
            saver::SaveError::DuplicateLog(name, match_id) => ApiError::Conflict {
                error: "logs.duplicate_log".to_string(),
                details: format!("Log \"{}\" already loaded for match #{}", name, match_id),
            },
            saver::SaveError::UnsupportedLogVersion(got, expected) => ApiError::BadRequest {
                error: "logs.unsupported_version".to_string(),
                details: format!(
                    "Log version \"{}\" is not supported (expected \"{}\")",
                    got, expected
                ),
            },
            saver::SaveError::TeamNotFound(team_name, tournament_id) => ApiError::BadRequest {
                error: "logs.team_not_found".to_string(),
                details: format!(
                    "Team \"{}\" not found in tournament #{}",
                    team_name, tournament_id
                ),
            },
            saver::SaveError::PlayerNotFound(team, player) => ApiError::BadRequest {
                error: "logs.player_not_found".to_string(),
                details: format!("Player \"{}\" not found in team \"{}\"", player, team),
            },
            saver::SaveError::MapNotFound(alias) => ApiError::BadRequest {
                error: "logs.map_not_found".to_string(),
                details: format!("Map \"{}\" not found", alias),
            },
            other => {
                error!("failed to save events: {:?}", other);
                ApiError::InternalError {
                    error: "internal_error".to_string(),
                }
            }
        })?;

    Ok(HttpResponse::NoContent().finish())
}
