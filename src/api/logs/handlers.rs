use crate::dal::Dal;
use crate::logs_parser::{events_parser, saver};
use actix_web::{post, web, HttpResponse, Responder, Result};
use log::error;
use std::sync::Arc;

#[post("/load/{match_id}")]
async fn load_log(
    match_id: web::Path<i64>,
    body: web::Bytes,
    db: web::Data<Arc<Dal>>,
) -> Result<impl Responder> {
    let match_row = db
        .matches
        .get_by_id(*match_id)
        .await
        .map_err(|e| actix_web::error::ErrorInternalServerError(e.to_string()))?
        .ok_or_else(|| {
            actix_web::error::ErrorNotFound(format!(
                "Match with id {} not found",
                match_id
            ))
        })?;

    let csv = String::from_utf8(body.to_vec())
        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

    let events = events_parser::parse_csv(&csv)
        .map_err(|e| actix_web::error::ErrorBadRequest(e.to_string()))?;

    saver::save_events(&db, &match_row, &events)
        .await
        .map_err(|e| match e {
            saver::SaveError::TeamNotFound(team_name, tournament_id) => {
                actix_web::error::ErrorBadRequest(format!(
                    "Команда с названием \"{}\" не найдена в турнире #{}",
                    team_name, tournament_id
                ))
            }
            saver::SaveError::PlayerNotFound(team, player) => {
                actix_web::error::ErrorBadRequest(format!(
                    "Игрок с ником \"{}\" не найден в команде \"{}\"",
                    player,
                    team,
                ))
            }
            other => {
                error!("failed to save events: {:?}", other);
                actix_web::error::ErrorInternalServerError("failed to save events")
            }
        })?;

    Ok(HttpResponse::NoContent().finish())
}
