use actix_web::{HttpResponse, Responder, get, web};
use chrono::NaiveDateTime;
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use crate::api::auth::jwt::AuthenticatedUser;
use crate::api::error::ApiError;
use crate::dal::{Dal, RoleValue};

#[derive(sqlx::FromRow)]
struct RosterBaseRow {
    registration_id: i64,
    battletag: String,
    primary_role: Option<RoleValue>,
    secondary_role: Option<RoleValue>,
    checked_in: Option<bool>,
    checked_in_at: Option<NaiveDateTime>,
    primary_role_override: Option<String>,
    secondary_role_override: Option<String>,
    role_rankings_override_json: Option<String>,
    full_flex_override: Option<bool>,
}

#[derive(sqlx::FromRow)]
struct RankingRow {
    registration_id: i64,
    role: RoleValue,
    ranking: i64,
}

#[derive(Serialize)]
struct CheckinOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    primary_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    secondary_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    role_rankings: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    full_flex: Option<bool>,
}

#[derive(Serialize)]
pub struct RosterEntry {
    registration_id: i64,
    battletag: String,
    primary_role: Option<RoleValue>,
    secondary_role: Option<RoleValue>,
    is_full_flex: bool,
    role_rankings: HashMap<String, i64>,
    checked_in: bool,
    checked_in_at: Option<NaiveDateTime>,
    overrides: Option<CheckinOverrides>,
}

pub fn configure_nested(cfg: &mut web::ServiceConfig) {
    cfg.service(get_roster);
}

#[get("/{tournament_id}/roster")]
pub async fn get_roster(
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

    let base_rows = sqlx::query_as::<_, RosterBaseRow>(
        r#"
        SELECT
            r.id AS registration_id,
            ub.battletag,
            r.primary_role,
            r.secondary_role,
            rc.checked_in,
            rc.checked_in_at,
            rc.primary_role_override,
            rc.secondary_role_override,
            rc.role_rankings_override_json,
            rc.full_flex_override
        FROM registrations r
        INNER JOIN user_battletags ub ON ub.id = r.user_battletag_id
        LEFT JOIN registration_checkins rc ON rc.registration_id = r.id
        WHERE r.tournament_id = ?1
          AND r.status = 'ACCEPTED'
        ORDER BY r.id
        "#,
    )
    .bind(tournament_id)
    .fetch_all(&db.db_pool)
    .await
    .map_err(|e: sqlx::Error| {
        log::error!("Failed to load roster for tournament {}: {}", tournament_id, e);
        ApiError::InternalError {
            error: e.to_string(),
        }
    })?;

    let ranking_rows = sqlx::query_as::<_, RankingRow>(
        r#"
        SELECT rr.registration_id, rr.role, rr.ranking
        FROM registration_role_rankings rr
        INNER JOIN registrations r ON r.id = rr.registration_id
        WHERE r.tournament_id = ?1
          AND r.status = 'ACCEPTED'
        ORDER BY rr.registration_id, rr.role
        "#,
    )
    .bind(tournament_id)
    .fetch_all(&db.db_pool)
    .await
    .map_err(|e: sqlx::Error| {
        log::error!(
            "Failed to load role rankings for tournament {}: {}",
            tournament_id,
            e
        );
        ApiError::InternalError {
            error: e.to_string(),
        }
    })?;

    // Group rankings by registration_id
    let mut rankings_by_reg: HashMap<i64, HashMap<String, i64>> = HashMap::new();
    for row in ranking_rows {
        let role_str = match row.role {
            RoleValue::Tank => "tank",
            RoleValue::Damage => "damage",
            RoleValue::Support => "support",
            RoleValue::Flex => "flex",
        };
        rankings_by_reg
            .entry(row.registration_id)
            .or_default()
            .insert(role_str.to_string(), row.ranking);
    }

    let roster: Vec<RosterEntry> = base_rows
        .into_iter()
        .map(|row| {
            let checked_in = row.checked_in.unwrap_or(false);
            let is_full_flex = row
                .full_flex_override
                .unwrap_or_else(|| row.primary_role == Some(RoleValue::Flex));

            let mut role_rankings: HashMap<String, i64> = HashMap::new();
            role_rankings.insert("tank".to_string(), 0);
            role_rankings.insert("damage".to_string(), 0);
            role_rankings.insert("support".to_string(), 0);
            role_rankings.insert("flex".to_string(), 0);
            if let Some(reg_rankings) = rankings_by_reg.get(&row.registration_id) {
                for (role, rank) in reg_rankings {
                    role_rankings.insert(role.clone(), *rank);
                }
            }

            // Build overrides if any override field is set
            let has_overrides = row.primary_role_override.is_some()
                || row.secondary_role_override.is_some()
                || row.role_rankings_override_json.is_some()
                || row.full_flex_override.is_some();

            let overrides = if has_overrides {
                let role_rankings_override = row
                    .role_rankings_override_json
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok());
                Some(CheckinOverrides {
                    primary_role: row.primary_role_override,
                    secondary_role: row.secondary_role_override,
                    role_rankings: role_rankings_override,
                    full_flex: row.full_flex_override,
                })
            } else {
                None
            };

            RosterEntry {
                registration_id: row.registration_id,
                battletag: row.battletag,
                primary_role: row.primary_role,
                secondary_role: row.secondary_role,
                is_full_flex,
                role_rankings,
                checked_in,
                checked_in_at: row.checked_in_at,
                overrides,
            }
        })
        .collect();

    Ok(HttpResponse::Ok().json(json!({ "items": roster })))
}
