use actix_web::{HttpResponse, Responder, get, web};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::api::auth::jwt::AuthenticatedUser;
use crate::api::error::ApiError;
use crate::dal::Dal;

#[derive(Debug, Deserialize)]
struct UserSearchQuery {
    search: Option<String>,
    limit: Option<i64>,
}

#[derive(Debug, Serialize)]
struct UserSearchResult {
    id: i64,
    battletag: Option<String>,
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/users").service(search_users));
}

#[get("")]
async fn search_users(
    _user: AuthenticatedUser,
    query: web::Query<UserSearchQuery>,
    db: web::Data<Arc<Dal>>,
) -> actix_web::Result<impl Responder, ApiError> {
    let limit = query.limit.unwrap_or(10).clamp(1, 20);
    let search = query.search.as_deref();

    let users = db
        .users
        .list_with_battletags(search, limit, 0)
        .await
        .map_err(|e| {
            log::error!("Failed to search users: {}", e);
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?;

    let results = users
        .into_iter()
        .map(|u| UserSearchResult {
            id: u.id,
            battletag: u.battletag,
        })
        .collect::<Vec<_>>();

    Ok(HttpResponse::Ok().json(results))
}
