use crate::api::error::ApiError;
use actix_web::{HttpResponse, Responder, get, web};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

use super::models::{Tournament, TournamentShort};

#[get("")]
pub async fn get_tournaments() -> actix_web::Result<impl Responder, ApiError> {
    Ok(HttpResponse::Ok().json(json!(vec![
        TournamentShort {
            title: "День Защитника Пейлоада",
            uri: "2026-02-defender-of-the-payload-day",
            dates: vec!["2026-02-21", "2026-02-22"],
            prize_pool: Some("40000 RUB")
        }
    ])))
}

#[get("/{id}")]
pub async fn get_tournament(id: web::Path<String>) -> actix_web::Result<impl Responder, ApiError> {
    let id = id.into_inner();
    
    // Basic path traversal protection
    if id.contains("..") || id.contains('/') || id.contains('\\') {
        return Err(ApiError::BadRequest {
            error: "Invalid ID".to_string(),
            details: "ID contains invalid characters".to_string(),
        });
    }

    let mut path = PathBuf::from("./static/tournaments");
    path.push(format!("{}.json", id));

    match fs::read_to_string(path) {
        Ok(content) => {
            let tournament: Tournament = serde_json::from_str(&content).map_err(|e| {
                log::error!("Failed to parse tournament JSON: {}", e);
                ApiError::InternalError {
                    error: e.to_string(),
                }
            })?;
            Ok(HttpResponse::Ok().json(tournament))
        }
        Err(_) => Err(ApiError::NotFound),
    }
}
