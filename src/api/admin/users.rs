use actix_web::{HttpResponse, Responder, delete, get, post, web};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::api::auth::jwt::AuthenticatedUser;
use crate::api::error::ApiError;
use crate::dal::Dal;
use crate::services::audit_service::{AuditActor, kinds, log_event};

#[derive(Debug, Deserialize)]
pub struct AdminUsersQuery {
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct GrantAuthorityRequest {
    pub authority: String,
}

#[derive(Debug, Serialize)]
pub struct AdminUserResponse {
    pub id: i64,
    pub battletag: Option<String>,
    pub is_admin: bool,
    pub is_banned: bool,
    pub authorities: Vec<String>,
}

#[derive(Debug, Serialize)]
struct AuthorityMutationResponse {
    pub user_id: i64,
    pub authority: String,
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/users")
            .service(list_users)
            .service(grant_authority)
            .service(revoke_authority),
    );
}

#[get("")]
pub async fn list_users(
    user: AuthenticatedUser,
    db: web::Data<Arc<Dal>>,
    query: web::Query<AdminUsersQuery>,
) -> actix_web::Result<impl Responder, ApiError> {
    if !user.is_admin() {
        return Err(ApiError::Forbidden);
    }

    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let offset = query.offset.unwrap_or(0).max(0);

    let rows = db
        .users
        .list_with_battletags(query.search.as_deref(), limit, offset)
        .await
        .map_err(|e| {
            log::error!("Failed to list users for admin UI: {}", e);
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?;

    let users = rows
        .into_iter()
        .map(|row| {
            let authorities = serde_json::from_str::<Vec<String>>(&row.authorities_json)
                .unwrap_or_else(|_| Vec::new());

            AdminUserResponse {
                id: row.id,
                battletag: row.battletag,
                is_admin: row.is_admin,
                is_banned: row.is_banned,
                authorities,
            }
        })
        .collect::<Vec<_>>();

    Ok(HttpResponse::Ok().json(users))
}

#[post("/{user_id}/authorities")]
pub async fn grant_authority(
    user: AuthenticatedUser,
    path: web::Path<i64>,
    payload: web::Json<GrantAuthorityRequest>,
    db: web::Data<Arc<Dal>>,
) -> actix_web::Result<impl Responder, ApiError> {
    if !user.is_admin() {
        return Err(ApiError::Forbidden);
    }

    let actor_user_id = parse_user_id(&user)?;
    let target_user_id = path.into_inner();
    const VALID_AUTHORITIES: &[&str] = &["create_tournament"];

    let authority = payload.authority.trim().to_string();
    if authority.is_empty() {
        return Err(ApiError::BadRequest {
            error: "invalid_authority".to_string(),
            details: "authority must not be empty".to_string(),
        });
    }
    if !VALID_AUTHORITIES.contains(&authority.as_str()) {
        return Err(ApiError::BadRequest {
            error: "unknown_authority".to_string(),
            details: format!(
                "unknown authority '{}'. valid: {}",
                authority,
                VALID_AUTHORITIES.join(", ")
            ),
        });
    }

    ensure_user_exists(&db, target_user_id).await?;

    db.users
        .grant_authority(target_user_id, &authority, actor_user_id)
        .await
        .map_err(|e| {
            log::error!(
                "Failed to grant authority {} to user {}: {}",
                authority,
                target_user_id,
                e
            );
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?;

    log_event(
        &db,
        &AuditActor {
            user_id: actor_user_id,
            battletag: user.battletag.clone(),
        },
        kinds::USER_AUTHORITY_GRANTED,
        target_user_id,
        None,
        json!({ "authority": authority }),
    )
    .await;

    Ok(HttpResponse::Ok().json(AuthorityMutationResponse {
        user_id: target_user_id,
        authority,
    }))
}

#[delete("/{user_id}/authorities/{authority}")]
pub async fn revoke_authority(
    user: AuthenticatedUser,
    path: web::Path<(i64, String)>,
    db: web::Data<Arc<Dal>>,
) -> actix_web::Result<impl Responder, ApiError> {
    if !user.is_admin() {
        return Err(ApiError::Forbidden);
    }

    let actor_user_id = parse_user_id(&user)?;
    let (target_user_id, authority) = path.into_inner();

    ensure_user_exists(&db, target_user_id).await?;

    db.users
        .revoke_authority(target_user_id, &authority)
        .await
        .map_err(|e| {
            log::error!(
                "Failed to revoke authority {} for user {}: {}",
                authority,
                target_user_id,
                e
            );
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?;

    log_event(
        &db,
        &AuditActor {
            user_id: actor_user_id,
            battletag: user.battletag.clone(),
        },
        kinds::USER_AUTHORITY_REVOKED,
        target_user_id,
        None,
        json!({ "authority": authority }),
    )
    .await;

    Ok(HttpResponse::Ok().json(AuthorityMutationResponse {
        user_id: target_user_id,
        authority,
    }))
}

fn parse_user_id(user: &AuthenticatedUser) -> Result<i64, ApiError> {
    user.id.parse::<i64>().map_err(|_| ApiError::BadRequest {
        error: "invalid_user_id".to_string(),
        details: "User id from auth token is not a number".to_string(),
    })
}

async fn ensure_user_exists(db: &Dal, user_id: i64) -> Result<(), ApiError> {
    let exists = db.users.find_by_id(user_id).await.map_err(|e| {
        log::error!("Failed to load user {}: {}", user_id, e);
        ApiError::InternalError {
            error: e.to_string(),
        }
    })?;

    if exists.is_none() {
        return Err(ApiError::NotFound);
    }

    Ok(())
}
