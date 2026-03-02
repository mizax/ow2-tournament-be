use actix_web::{HttpResponse, Responder, get, post, web};
use serde::Deserialize;
use std::sync::Arc;

use crate::api::auth::jwt::AuthenticatedUser;
use crate::api::error::ApiError;
use crate::dal::Dal;

#[derive(Debug, Deserialize)]
pub struct SaveBalanceRequest {
    pub payload: serde_json::Value,
}

pub fn configure_nested(cfg: &mut web::ServiceConfig) {
    cfg.service(save_balance).service(list_balances);
}

#[post("/{tournament_id}/balances")]
pub async fn save_balance(
    user: AuthenticatedUser,
    db: web::Data<Arc<Dal>>,
    tournament_id: web::Path<i64>,
    payload: web::Json<SaveBalanceRequest>,
) -> actix_web::Result<impl Responder, ApiError> {
    let user_id = user.id.parse::<i64>().map_err(|_| ApiError::BadRequest {
        error: "Invalid user id".to_string(),
        details: "User id from auth token is not a number.".to_string(),
    })?;
    let tournament_id = tournament_id.into_inner();

    let is_manager = db
        .tournament_managers
        .user_is_manager_for_tournament(tournament_id, user_id)
        .await
        .map_err(|e| {
            log::error!("Failed to check manager status: {}", e);
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?;
    if !is_manager {
        return Err(ApiError::Forbidden);
    }

    let balance_id = db
        .balances
        .save(tournament_id, user_id, payload.into_inner().payload)
        .await
        .map_err(|e| {
            log::error!(
                "Failed to save balance for tournament {}: {}",
                tournament_id,
                e
            );
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?;

    let balance = db
        .balances
        .get(balance_id)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch saved balance {}: {}", balance_id, e);
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?
        .ok_or(ApiError::NotFound)?;

    Ok(HttpResponse::Created().json(balance))
}

#[get("/{tournament_id}/balances")]
pub async fn list_balances(
    user: AuthenticatedUser,
    db: web::Data<Arc<Dal>>,
    tournament_id: web::Path<i64>,
) -> actix_web::Result<impl Responder, ApiError> {
    let user_id = user.id.parse::<i64>().map_err(|_| ApiError::BadRequest {
        error: "Invalid user id".to_string(),
        details: "User id from auth token is not a number.".to_string(),
    })?;
    let tournament_id = tournament_id.into_inner();

    let is_manager = db
        .tournament_managers
        .user_is_manager_for_tournament(tournament_id, user_id)
        .await
        .map_err(|e| {
            log::error!("Failed to check manager status: {}", e);
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?;
    if !is_manager {
        return Err(ApiError::Forbidden);
    }

    let balances = db
        .balances
        .list_for_tournament(tournament_id)
        .await
        .map_err(|e| {
            log::error!(
                "Failed to list balances for tournament {}: {}",
                tournament_id,
                e
            );
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?;

    Ok(HttpResponse::Ok().json(serde_json::json!({ "items": balances })))
}
