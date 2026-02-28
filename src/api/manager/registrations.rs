use actix_web::{HttpResponse, Responder, get, patch, post, web};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;

use crate::api::auth::jwt::AuthenticatedUser;
use crate::api::error::ApiError;
use crate::dal::{
    Dal, RegistrationComment, RegistrationRequestedAction, RegistrationRoleRanking,
    RegistrationRow, RegistrationSortField, RegistrationStatus, RegistrationSummary, RoleValue,
    SortDirection,
};
use crate::services::{
    AuditActor, GeoIpInfo, GeoIpService, audit_change, audit_kinds, log_audit_event,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagerRegistration {
    pub id: i64,
    pub tournament_id: i64,
    pub user_id: i64,
    pub user_battletag_id: i64,
    pub status: RegistrationStatus,
    pub alt_accounts: Option<Vec<String>>,
    pub twitch: String,
    pub discord: String,
    pub primary_role: Option<RoleValue>,
    pub secondary_role: Option<RoleValue>,
    pub guarantors: Option<Vec<String>>,
    pub additional_info: String,
    pub rules_accepted: bool,
    pub ip_address: String,
    pub geo_ip: Option<GeoIpInfo>,
    pub user_agent: String,
    pub decline_reason: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistrationDetailResponse {
    pub registration: ManagerRegistration,
    pub battletag: String,
    pub comments: Vec<RegistrationComment>,
    pub requested_actions: Vec<RegistrationRequestedAction>,
    pub role_rankings: Vec<RegistrationRoleRanking>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegistrationListResponse {
    pub items: Vec<RegistrationSummary>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateCommentRequest {
    pub comment: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateRegistrationStatusRequest {
    pub status: RegistrationStatus,
    pub decline_reason: Option<String>,
    pub requested_action_description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoleRankingAssignment {
    pub role: RoleValue,
    pub ranking: i64,
}

#[derive(Debug, Deserialize)]
struct RoleRankingUpdateRequest {
    pub role_assignments: Vec<RoleRankingAssignment>,
}

#[derive(Debug, Deserialize)]
struct RegistrationListQuery {
    pub tournament_id: i64,
    pub status: Option<String>,
    pub sort: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
    pub battletag: Option<String>,
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/registrations")
            .service(list_registrations)
            .service(get_registration)
            .service(add_comment)
            .service(update_status)
            .service(update_role_rankings)
            .service(resolve_action),
    );
}

#[get("/")]
pub async fn list_registrations(
    user: AuthenticatedUser,
    db: web::Data<Arc<Dal>>,
    query: web::Query<RegistrationListQuery>,
) -> actix_web::Result<impl Responder, ApiError> {
    let user_id = user.id.parse::<i64>().map_err(|_| ApiError::BadRequest {
        error: "Invalid user id".to_string(),
        details: "User id from auth token is not a number.".to_string(),
    })?;

    if query.tournament_id <= 0 {
        return Err(ApiError::BadRequest {
            error: "Invalid tournament id".to_string(),
            details: "Tournament id must be a positive number.".to_string(),
        });
    }

    let is_manager = db
        .tournament_managers
        .user_is_manager_for_tournament(query.tournament_id, user_id)
        .await
        .map_err(|e| {
            log::error!(
                "Failed to check manager status for user {} in tournament {}: {}",
                user_id,
                query.tournament_id,
                e
            );
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?;

    if !is_manager {
        return Err(ApiError::Forbidden);
    }

    let (sort_field, sort_direction) = parse_sort(query.sort.as_deref())?;
    let statuses = match query.status.as_deref() {
        Some(raw) => Some(parse_statuses(raw)?),
        None => None,
    };

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).clamp(1, 100);
    let offset = (page - 1) * per_page;
    let battletag_pattern = query
        .battletag
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("%{}%", value));

    let total = db
        .registrations
        .count_manager_summaries(
            query.tournament_id,
            statuses.as_deref(),
            battletag_pattern.clone(),
        )
        .await
        .map_err(|e| {
            log::error!(
                "Failed to count registrations for tournament {}: {}",
                query.tournament_id,
                e
            );
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?;

    let registrations = db
        .registrations
        .list_manager_summaries(
            query.tournament_id,
            statuses.as_deref(),
            sort_field,
            sort_direction,
            offset,
            per_page,
            battletag_pattern,
        )
        .await
        .map_err(|e| {
            log::error!(
                "Failed to load registrations for tournament {}: {}",
                query.tournament_id,
                e
            );
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?;

    Ok(HttpResponse::Ok().json(RegistrationListResponse {
        items: registrations,
        total,
        page,
        per_page,
    }))
}

#[get("/{registration_id}")]
pub async fn get_registration(
    user: AuthenticatedUser,
    db: web::Data<Arc<Dal>>,
    geo_ip_service: web::Data<GeoIpService>,
    registration_id: web::Path<i64>,
) -> actix_web::Result<impl Responder, ApiError> {
    let user_id = user.id.parse::<i64>().map_err(|_| ApiError::BadRequest {
        error: "Invalid user id".to_string(),
        details: "User id from auth token is not a number.".to_string(),
    })?;
    let registration_id = registration_id.into_inner();

    if !is_manager_for_registration(&db, user_id, registration_id).await? {
        return Err(ApiError::Forbidden);
    }

    let detail = db
        .registrations
        .get_manager_detail(registration_id)
        .await
        .map_err(|e| {
            log::error!("Failed to load registration {}: {}", registration_id, e);
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?
        .ok_or(ApiError::NotFound)?;

    let mut registration = map_registration(detail.registration)?;
    registration.geo_ip = geo_ip_service.lookup_ip(&registration.ip_address).await;

    Ok(HttpResponse::Ok().json(RegistrationDetailResponse {
        registration,
        battletag: detail.battletag,
        comments: detail.comments,
        requested_actions: detail.requested_actions,
        role_rankings: detail.role_rankings,
    }))
}

#[post("/{registration_id}/comments")]
pub async fn add_comment(
    user: AuthenticatedUser,
    db: web::Data<Arc<Dal>>,
    registration_id: web::Path<i64>,
    payload: web::Json<CreateCommentRequest>,
) -> actix_web::Result<impl Responder, ApiError> {
    let user_id = user.id.parse::<i64>().map_err(|_| ApiError::BadRequest {
        error: "Invalid user id".to_string(),
        details: "User id from auth token is not a number.".to_string(),
    })?;
    let registration_id = registration_id.into_inner();

    if !is_manager_for_registration(&db, user_id, registration_id).await? {
        return Err(ApiError::Forbidden);
    }

    let comment = payload.comment.trim();
    if comment.is_empty() {
        return Err(ApiError::ValidationError {
            errors: vec!["validation.comment.required".to_string()],
        });
    }

    let comment = db
        .registrations
        .create_comment(registration_id, user_id, comment.to_string())
        .await
        .map_err(|e| {
            log::error!(
                "Failed to create comment for registration {}: {}",
                registration_id,
                e
            );
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?;

    Ok(HttpResponse::Ok().json(comment))
}

#[patch("/{registration_id}/status")]
pub async fn update_status(
    user: AuthenticatedUser,
    db: web::Data<Arc<Dal>>,
    geo_ip_service: web::Data<GeoIpService>,
    registration_id: web::Path<i64>,
    payload: web::Json<UpdateRegistrationStatusRequest>,
) -> actix_web::Result<impl Responder, ApiError> {
    let user_id = user.id.parse::<i64>().map_err(|_| ApiError::BadRequest {
        error: "Invalid user id".to_string(),
        details: "User id from auth token is not a number.".to_string(),
    })?;
    let registration_id = registration_id.into_inner();

    if !is_manager_for_registration(&db, user_id, registration_id).await? {
        return Err(ApiError::Forbidden);
    }

    let old_status = db
        .registrations
        .get_status(registration_id)
        .await
        .map_err(|e| {
            log::error!(
                "Failed to fetch current status for registration {}: {}",
                registration_id,
                e
            );
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?
        .ok_or(ApiError::NotFound)?;

    let mut validation_errors = Vec::new();
    // if payload.status == RegistrationStatus::Declined {
    //     if payload
    //         .decline_reason
    //         .as_ref()
    //         .map(|value| value.trim().is_empty())
    //         .unwrap_or(true)
    //     {
    //         validation_errors.push("validation.decline_reason.required".to_string());
    //     }
    // }
    if payload.status == RegistrationStatus::ActionRequired {
        if payload
            .requested_action_description
            .as_ref()
            .map(|value| value.trim().is_empty())
            .unwrap_or(true)
        {
            validation_errors.push("validation.requested_action_description.required".to_string());
        }
    }
    if !validation_errors.is_empty() {
        return Err(ApiError::ValidationError {
            errors: validation_errors,
        });
    }

    let decline_reason = if payload.status == RegistrationStatus::Declined {
        payload
            .decline_reason
            .as_ref()
            .map(|value| value.trim().to_string())
    } else {
        None
    };

    let requested_action_description = if payload.status == RegistrationStatus::ActionRequired {
        payload
            .requested_action_description
            .as_ref()
            .map(|value| value.trim().to_string())
    } else {
        None
    };

    db.registrations
        .update_status_with_action(
            registration_id,
            payload.status,
            decline_reason.clone(),
            requested_action_description,
            user_id,
        )
        .await
        .map_err(|e| {
            log::error!("Failed to update registration {}: {}", registration_id, e);
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?;

    let detail = db
        .registrations
        .get_manager_detail(registration_id)
        .await
        .map_err(|e| {
            log::error!(
                "Failed to load updated registration {}: {}",
                registration_id,
                e
            );
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?
        .ok_or(ApiError::NotFound)?;

    let tournament_id = detail.registration.tournament_id;
    let mut registration = map_registration(detail.registration)?;
    registration.geo_ip = geo_ip_service.lookup_ip(&registration.ip_address).await;

    log_audit_event(
        &db,
        &AuditActor {
            user_id,
            battletag: user.battletag.clone(),
        },
        audit_kinds::REGISTRATION_STATUS_CHANGED,
        registration_id,
        Some(tournament_id),
        json!({
            "status": audit_change(
                serde_json::to_value(old_status).unwrap_or(serde_json::Value::Null),
                serde_json::to_value(payload.status).unwrap_or(serde_json::Value::Null)
            ),
            "decline_reason": decline_reason
        }),
    )
    .await;

    Ok(HttpResponse::Ok().json(RegistrationDetailResponse {
        registration,
        battletag: detail.battletag,
        comments: detail.comments,
        requested_actions: detail.requested_actions,
        role_rankings: detail.role_rankings,
    }))
}

#[patch("/{registration_id}/role-rankings")]
pub async fn update_role_rankings(
    user: AuthenticatedUser,
    db: web::Data<Arc<Dal>>,
    registration_id: web::Path<i64>,
    payload: web::Json<RoleRankingUpdateRequest>,
) -> actix_web::Result<impl Responder, ApiError> {
    let user_id = user.id.parse::<i64>().map_err(|_| ApiError::BadRequest {
        error: "Invalid user id".to_string(),
        details: "User id from auth token is not a number.".to_string(),
    })?;
    let registration_id = registration_id.into_inner();

    if !is_manager_for_registration(&db, user_id, registration_id).await? {
        return Err(ApiError::Forbidden);
    }

    let tournament_id = db
        .registrations
        .get_tournament_id(registration_id)
        .await
        .map_err(|e| {
            log::error!(
                "Failed to fetch tournament id for registration {}: {}",
                registration_id,
                e
            );
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?
        .ok_or(ApiError::NotFound)?;

    let mut seen_roles = HashSet::new();
    let mut validation_errors = Vec::new();
    for assignment in &payload.role_assignments {
        if assignment.ranking < 0 {
            validation_errors.push("validation.role_ranking.negative".to_string());
        }
        if !seen_roles.insert(assignment.role) {
            validation_errors.push("validation.role_ranking.duplicate_role".to_string());
        }
    }
    if !validation_errors.is_empty() {
        return Err(ApiError::ValidationError {
            errors: validation_errors,
        });
    }

    let rankings = payload
        .role_assignments
        .iter()
        .map(|assignment| (assignment.role, assignment.ranking))
        .collect::<Vec<_>>();

    let updated = db
        .registrations
        .replace_role_rankings(registration_id, rankings)
        .await
        .map_err(|e| {
            log::error!(
                "Failed to update role rankings for registration {}: {}",
                registration_id,
                e
            );
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?;

    log_audit_event(
        &db,
        &AuditActor {
            user_id,
            battletag: user.battletag.clone(),
        },
        audit_kinds::REGISTRATION_ROLE_RANKINGS_UPDATED,
        registration_id,
        Some(tournament_id),
        json!({
            "rankings": updated
                .iter()
                .map(|row| json!({ "role": row.role, "ranking": row.ranking }))
                .collect::<Vec<_>>()
        }),
    )
    .await;

    Ok(HttpResponse::Ok().json(updated))
}

#[patch("/{registration_id}/actions/{action_id}/resolve")]
pub async fn resolve_action(
    user: AuthenticatedUser,
    db: web::Data<Arc<Dal>>,
    path: web::Path<(i64, i64)>,
) -> actix_web::Result<impl Responder, ApiError> {
    let user_id = user.id.parse::<i64>().map_err(|_| ApiError::BadRequest {
        error: "Invalid user id".to_string(),
        details: "User id from auth token is not a number.".to_string(),
    })?;
    let (registration_id, action_id) = path.into_inner();

    if !is_manager_for_registration(&db, user_id, registration_id).await? {
        return Err(ApiError::Forbidden);
    }

    let action = db
        .registrations
        .resolve_requested_action(registration_id, action_id)
        .await
        .map_err(|e| {
            log::error!(
                "Failed to resolve action {} for registration {}: {}",
                action_id,
                registration_id,
                e
            );
            ApiError::InternalError {
                error: e.to_string(),
            }
        })?
        .ok_or(ApiError::NotFound)?;

    Ok(HttpResponse::Ok().json(action))
}

async fn is_manager_for_registration(
    db: &web::Data<Arc<Dal>>,
    user_id: i64,
    registration_id: i64,
) -> Result<bool, ApiError> {
    db.tournament_managers
        .user_is_manager_for_registration(registration_id, user_id)
        .await
        .map_err(|e| {
            log::error!(
                "Failed to check manager status for user {} on registration {}: {}",
                user_id,
                registration_id,
                e
            );
            ApiError::InternalError {
                error: e.to_string(),
            }
        })
}

fn parse_sort(sort: Option<&str>) -> Result<(RegistrationSortField, SortDirection), ApiError> {
    let sort = sort.unwrap_or("created_at:desc");
    let mut parts = sort.split(':');
    let field = parts.next().unwrap_or("created_at");
    let direction = parts.next().unwrap_or("desc");

    let field = match field {
        "created_at" => RegistrationSortField::CreatedAt,
        "updated_at" => RegistrationSortField::UpdatedAt,
        _ => {
            return Err(ApiError::BadRequest {
                error: "Invalid sort".to_string(),
                details: "Sort field must be created_at or updated_at.".to_string(),
            });
        }
    };

    let direction = match direction {
        "asc" => SortDirection::Asc,
        "desc" => SortDirection::Desc,
        _ => {
            return Err(ApiError::BadRequest {
                error: "Invalid sort".to_string(),
                details: "Sort direction must be asc or desc.".to_string(),
            });
        }
    };

    Ok((field, direction))
}

fn parse_statuses(statuses: &str) -> Result<Vec<RegistrationStatus>, ApiError> {
    let mut parsed = Vec::new();
    for raw in statuses.split(',') {
        let normalized = raw.trim().to_uppercase();
        let status = match normalized.as_str() {
            "PENDING" => RegistrationStatus::Pending,
            "PROCESSING" => RegistrationStatus::Processing,
            "ACCEPTED" => RegistrationStatus::Accepted,
            "ACTION_REQUIRED" => RegistrationStatus::ActionRequired,
            "DECLINED" => RegistrationStatus::Declined,
            "DELETED" => RegistrationStatus::Deleted,
            _ => {
                return Err(ApiError::BadRequest {
                    error: "Invalid status".to_string(),
                    details: format!("Unsupported status value: {}", raw),
                });
            }
        };
        parsed.push(status);
    }

    Ok(parsed)
}

fn map_registration(row: RegistrationRow) -> Result<ManagerRegistration, ApiError> {
    let alt_accounts = parse_optional_list(&row.alt_accounts_json)?;
    let guarantors = parse_optional_list(&row.guarantors_json)?;

    Ok(ManagerRegistration {
        id: row.id,
        tournament_id: row.tournament_id,
        user_id: row.user_id,
        user_battletag_id: row.user_battletag_id,
        status: row.status,
        alt_accounts,
        twitch: row.twitch,
        discord: row.discord,
        primary_role: row.primary_role,
        secondary_role: row.secondary_role,
        guarantors,
        additional_info: row.additional_info,
        rules_accepted: row.rules_accepted,
        ip_address: row.ip_address,
        geo_ip: None,
        user_agent: row.user_agent,
        decline_reason: row.decline_reason,
        created_at: row.created_at,
        updated_at: row.updated_at,
        version: row.version,
    })
}

fn parse_optional_list(payload: &str) -> Result<Option<Vec<String>>, ApiError> {
    let items: Vec<String> = serde_json::from_str(payload).map_err(|e| {
        log::error!("Failed to parse registration list payload: {}", e);
        ApiError::InternalError {
            error: e.to_string(),
        }
    })?;
    if items.is_empty() {
        Ok(None)
    } else {
        Ok(Some(items))
    }
}
