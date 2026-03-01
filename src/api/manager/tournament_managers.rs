use actix_web::{HttpResponse, Responder, delete, get, post, web};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;

use crate::api::auth::jwt::{AuthenticatedUser, UserRole};
use crate::api::error::ApiError;
use crate::dal::Dal;
use crate::services::audit_service::{AuditActor, kinds, log_event};

#[derive(Debug, Serialize)]
struct TournamentManagerResponse {
    pub user_id: i64,
    pub battletag: Option<String>,
    pub is_owner: bool,
    pub can_manage_managers: bool,
    pub added_by_user_id: Option<i64>,
    pub added_by_battletag: Option<String>,
    pub added_at: String,
    pub can_remove: bool,
}

#[derive(Debug, Deserialize)]
struct AddManagerRequest {
    user_id: i64,
    can_manage_managers: bool,
}

/// Called from within the `/tournaments` scope in tournaments.rs — prefix here is `/{tournament_id}/managers`.
pub fn configure_nested(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/{tournament_id}/managers")
            .service(list_managers)
            .service(add_manager)
            .service(remove_manager),
    );
}

#[get("")]
async fn list_managers(
    user: AuthenticatedUser,
    path: web::Path<i64>,
    db: web::Data<Arc<Dal>>,
) -> actix_web::Result<impl Responder, ApiError> {
    let tournament_id = path.into_inner();
    let caller_id = parse_user_id(&user)?;
    let is_admin = user.roles.contains(&UserRole::Admin);

    if !is_admin {
        let is_manager = db
            .tournament_managers
            .user_is_manager_for_tournament(tournament_id, caller_id)
            .await
            .map_err(db_error)?;
        if !is_manager {
            return Err(ApiError::Forbidden);
        }
    }

    let managers = db
        .tournament_managers
        .list_managers(tournament_id)
        .await
        .map_err(db_error)?;

    let caller_has_can_manage = if is_admin {
        true
    } else {
        managers
            .iter()
            .find(|m| m.user_id == caller_id)
            .map(|m| m.can_manage_managers)
            .unwrap_or(false)
    };

    let descendants: HashSet<i64> = if is_admin {
        HashSet::new()
    } else {
        db.tournament_managers
            .get_descendants(tournament_id, caller_id)
            .await
            .map_err(db_error)?
            .into_iter()
            .collect()
    };

    let response = managers
        .into_iter()
        .map(|m| {
            let can_remove = if is_admin {
                !m.is_owner && m.user_id != caller_id
            } else {
                caller_has_can_manage
                    && !m.is_owner
                    && m.user_id != caller_id
                    && descendants.contains(&m.user_id)
            };
            TournamentManagerResponse {
                user_id: m.user_id,
                battletag: m.battletag,
                is_owner: m.is_owner,
                can_manage_managers: m.can_manage_managers,
                added_by_user_id: m.added_by_user_id,
                added_by_battletag: m.added_by_battletag,
                added_at: m.added_at.to_string(),
                can_remove,
            }
        })
        .collect::<Vec<_>>();

    Ok(HttpResponse::Ok().json(response))
}

#[post("")]
async fn add_manager(
    user: AuthenticatedUser,
    path: web::Path<i64>,
    payload: web::Json<AddManagerRequest>,
    db: web::Data<Arc<Dal>>,
) -> actix_web::Result<impl Responder, ApiError> {
    let tournament_id = path.into_inner();
    let caller_id = parse_user_id(&user)?;
    let is_admin = user.roles.contains(&UserRole::Admin);

    if !is_admin {
        let can_manage = db
            .tournament_managers
            .user_can_manage_managers(tournament_id, caller_id)
            .await
            .map_err(db_error)?;
        if !can_manage {
            return Err(ApiError::Forbidden);
        }
    }

    let target_id = payload.user_id;

    db.users
        .find_by_id(target_id)
        .await
        .map_err(db_error)?
        .ok_or(ApiError::NotFound)?;

    let already_manager = db
        .tournament_managers
        .user_is_manager_for_tournament(tournament_id, target_id)
        .await
        .map_err(db_error)?;

    if already_manager {
        return Err(ApiError::Conflict {
            error: "already_manager".to_string(),
            details: "User is already a manager of this tournament".to_string(),
        });
    }

    db.tournament_managers
        .add_manager_by(
            tournament_id,
            target_id,
            caller_id,
            payload.can_manage_managers,
        )
        .await
        .map_err(db_error)?;

    log_event(
        &db,
        &AuditActor {
            user_id: caller_id,
            battletag: user.battletag.clone(),
        },
        kinds::TOURNAMENT_MANAGER_ADDED,
        tournament_id,
        Some(tournament_id),
        json!({
            "user_id": target_id,
            "can_manage_managers": payload.can_manage_managers,
            "added_by": caller_id,
        }),
    )
    .await;

    Ok(HttpResponse::Created().finish())
}

#[delete("/{target_user_id}")]
async fn remove_manager(
    user: AuthenticatedUser,
    path: web::Path<(i64, i64)>,
    db: web::Data<Arc<Dal>>,
) -> actix_web::Result<impl Responder, ApiError> {
    let (tournament_id, target_id) = path.into_inner();
    let caller_id = parse_user_id(&user)?;
    let is_admin = user.roles.contains(&UserRole::Admin);

    if !is_admin {
        let can_manage = db
            .tournament_managers
            .user_can_manage_managers(tournament_id, caller_id)
            .await
            .map_err(db_error)?;
        if !can_manage {
            return Err(ApiError::Forbidden);
        }
    }

    let managers = db
        .tournament_managers
        .list_managers(tournament_id)
        .await
        .map_err(db_error)?;

    let (target_is_owner, target_battletag) = managers
        .iter()
        .find(|m| m.user_id == target_id)
        .map(|m| (m.is_owner, m.battletag.clone()))
        .ok_or(ApiError::NotFound)?;

    if target_is_owner {
        return Err(ApiError::Forbidden);
    }

    if target_id == caller_id {
        return Err(ApiError::Forbidden);
    }

    if !is_admin {
        let is_ancestor = db
            .tournament_managers
            .caller_is_ancestor_of(tournament_id, caller_id, target_id)
            .await
            .map_err(db_error)?;
        if !is_ancestor {
            return Err(ApiError::Forbidden);
        }
    }

    db.tournament_managers
        .remove_manager(tournament_id, target_id)
        .await
        .map_err(db_error)?;

    log_event(
        &db,
        &AuditActor {
            user_id: caller_id,
            battletag: user.battletag.clone(),
        },
        kinds::TOURNAMENT_MANAGER_REMOVED,
        tournament_id,
        Some(tournament_id),
        json!({
            "user_id": target_id,
            "battletag": target_battletag,
            "removed_by": caller_id,
        }),
    )
    .await;

    Ok(HttpResponse::NoContent().finish())
}

fn parse_user_id(user: &AuthenticatedUser) -> Result<i64, ApiError> {
    user.id.parse::<i64>().map_err(|_| ApiError::BadRequest {
        error: "Invalid user id".to_string(),
        details: "User id from auth token is not a number.".to_string(),
    })
}

fn db_error(e: sqlx::Error) -> ApiError {
    log::error!("Database error: {}", e);
    ApiError::InternalError {
        error: e.to_string(),
    }
}
