use actix_web::{HttpResponse, Responder, patch, web};
use serde::Deserialize;
use std::sync::Arc;

use crate::api::auth::jwt::AuthenticatedUser;
use crate::api::error::ApiError;
use crate::dal::{CheckinUpsert, Dal};

#[derive(Debug, Deserialize)]
pub struct CheckinUpdateItem {
    pub registration_id: i64,
    pub checked_in: bool,
    pub primary_role_override: Option<String>,
    pub secondary_role_override: Option<String>,
    pub role_rankings_override_json: Option<String>,
    pub full_flex_override: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct BatchCheckinRequest {
    pub items: Vec<CheckinUpdateItem>,
}

pub fn configure_nested(cfg: &mut web::ServiceConfig) {
    cfg.service(batch_upsert).service(single_upsert);
}

#[patch("/{tournament_id}/checkins")]
pub async fn batch_upsert(
    user: AuthenticatedUser,
    db: web::Data<Arc<Dal>>,
    tournament_id: web::Path<i64>,
    payload: web::Json<BatchCheckinRequest>,
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

    let upserts: Vec<CheckinUpsert> = payload
        .items
        .iter()
        .map(|item| CheckinUpsert {
            registration_id: item.registration_id,
            checked_in: item.checked_in,
            primary_role_override: item.primary_role_override.clone(),
            secondary_role_override: item.secondary_role_override.clone(),
            role_rankings_override_json: item.role_rankings_override_json.clone(),
            full_flex_override: item.full_flex_override,
        })
        .collect();

    db.checkins
        .upsert_batch(tournament_id, user_id, upserts)
        .await
        .map_err(|e| {
            log::error!(
                "Failed to batch upsert checkins for tournament {}: {}",
                tournament_id,
                e
            );
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?;

    Ok(HttpResponse::NoContent().finish())
}

#[patch("/{tournament_id}/checkins/{reg_id}")]
pub async fn single_upsert(
    user: AuthenticatedUser,
    db: web::Data<Arc<Dal>>,
    path: web::Path<(i64, i64)>,
    payload: web::Json<CheckinUpdateItem>,
) -> actix_web::Result<impl Responder, ApiError> {
    let user_id = user.id.parse::<i64>().map_err(|_| ApiError::BadRequest {
        error: "Invalid user id".to_string(),
        details: "User id from auth token is not a number.".to_string(),
    })?;
    let (tournament_id, reg_id) = path.into_inner();

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

    let checkin = db
        .checkins
        .upsert_single(
            tournament_id,
            user_id,
            CheckinUpsert {
                registration_id: reg_id,
                checked_in: payload.checked_in,
                primary_role_override: payload.primary_role_override.clone(),
                secondary_role_override: payload.secondary_role_override.clone(),
                role_rankings_override_json: payload.role_rankings_override_json.clone(),
                full_flex_override: payload.full_flex_override,
            },
        )
        .await
        .map_err(|e| {
            log::error!(
                "Failed to upsert checkin for reg {} in tournament {}: {}",
                reg_id,
                tournament_id,
                e
            );
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?;

    Ok(HttpResponse::Ok().json(checkin))
}
