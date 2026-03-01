use actix_web::{HttpResponse, Responder, get, web};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

use crate::api::auth::jwt::{AuthenticatedUser, UserRole};
use crate::api::error::ApiError;
use crate::dal::{AuditFilter, Dal};

#[derive(Debug, Deserialize)]
pub struct AuditListQuery {
    pub entity_type: Option<String>,
    pub entity_id: Option<i64>,
    pub tournament_id: Option<i64>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AuditListItem {
    pub id: i64,
    pub created_at: NaiveDateTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_user_id: Option<i64>,
    pub actor_battletag: String,
    pub action: String,
    pub entity_type: String,
    pub entity_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tournament_id: Option<i64>,
    pub details: Value,
}

#[derive(Debug, Serialize)]
pub struct AuditListResponse {
    pub items: Vec<AuditListItem>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/audit").service(list_audit));
}

#[get("")]
pub async fn list_audit(
    user: AuthenticatedUser,
    db: web::Data<Arc<Dal>>,
    query: web::Query<AuditListQuery>,
) -> actix_web::Result<impl Responder, ApiError> {
    list_audit_impl(user, db, query).await
}

async fn list_audit_impl(
    user: AuthenticatedUser,
    db: web::Data<Arc<Dal>>,
    query: web::Query<AuditListQuery>,
) -> actix_web::Result<HttpResponse, ApiError> {
    if !user.roles.contains(&UserRole::Admin) {
        return Err(ApiError::Forbidden);
    }

    let page = query.page.unwrap_or(1);
    let per_page = query.per_page.unwrap_or(50);

    let entity_type = query
        .entity_type
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(|v| v.to_string());

    let (rows, total) = db
        .audit
        .list(AuditFilter {
            entity_type,
            entity_id: query.entity_id,
            tournament_id: query.tournament_id,
            page,
            per_page,
        })
        .await
        .map_err(|e| {
            log::error!("Failed to list audit logs: {}", e);
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?;

    let items = rows
        .into_iter()
        .map(|row| {
            let details = match serde_json::from_str::<Value>(&row.details_json) {
                Ok(value) => value,
                Err(error) => {
                    log::error!(
                        "Failed to parse audit details_json for row {}: {}",
                        row.id,
                        error
                    );
                    serde_json::json!({})
                }
            };

            AuditListItem {
                id: row.id,
                created_at: row.created_at,
                actor_user_id: row.actor_user_id,
                actor_battletag: row.actor_battletag,
                action: row.action,
                entity_type: row.entity_type,
                entity_id: row.entity_id,
                tournament_id: row.tournament_id,
                details,
            }
        })
        .collect::<Vec<_>>();

    Ok(HttpResponse::Ok().json(AuditListResponse {
        items,
        total,
        page,
        per_page,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::TestDb;
    use actix_web::body::to_bytes;

    #[actix_web::test]
    async fn list_audit_returns_403_for_non_admin() {
        let db = TestDb::new().await;
        let dal = Arc::new(Dal::from_pool(db.pool.clone()));

        let user = AuthenticatedUser {
            id: "10".to_string(),
            battletag: "User#1234".to_string(),
            roles: vec![UserRole::NormalUser],
            authorities: vec![],
        };

        let result = list_audit_impl(
            user,
            web::Data::new(dal),
            web::Query(AuditListQuery {
                entity_type: None,
                entity_id: None,
                tournament_id: None,
                page: None,
                per_page: None,
            }),
        )
        .await;

        assert!(matches!(result, Err(ApiError::Forbidden)));
    }

    #[actix_web::test]
    async fn list_audit_returns_items_for_admin() {
        let db = TestDb::new().await;
        sqlx::query(
            r#"
            INSERT INTO audit_log (
                actor_user_id, actor_battletag, action, entity_type, entity_id, tournament_id, details_json
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
        )
        .bind(42_i64)
        .bind("Admin#0001")
        .bind("tournament.updated")
        .bind("tournament")
        .bind(5_i64)
        .bind(5_i64)
        .bind(r#"{"changed_fields":{"title":{"old":"Old","new":"New"}}}"#)
        .execute(&db.pool)
        .await
        .expect("insert audit row");

        let dal = Arc::new(Dal::from_pool(db.pool.clone()));
        let user = AuthenticatedUser {
            id: "42".to_string(),
            battletag: "Admin#0001".to_string(),
            roles: vec![UserRole::Admin],
            authorities: vec![],
        };

        let response = list_audit_impl(
            user,
            web::Data::new(dal),
            web::Query(AuditListQuery {
                entity_type: Some("tournament".to_string()),
                entity_id: None,
                tournament_id: Some(5),
                page: Some(1),
                per_page: Some(50),
            }),
        )
        .await
        .expect("list audit as admin");
        assert_eq!(response.status(), 200);

        let body = to_bytes(response.into_body()).await.expect("read body");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("parse json");

        assert_eq!(json["total"], 1);
        assert_eq!(json["items"][0]["action"], "tournament.updated");
        assert_eq!(json["items"][0]["entity_type"], "tournament");
        assert_eq!(
            json["items"][0]["details"]["changed_fields"]["title"]["new"],
            "New"
        );
    }
}
