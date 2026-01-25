use actix_web::{HttpResponse, Responder, get, web};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::Arc;

use crate::api::auth::jwt::AuthenticatedUser;
use crate::api::error::ApiError;
use crate::dal::{Dal, RegistrationStatus, RoleValue};

#[derive(Serialize)]
pub struct RegistrationDetailsResponse {
    #[serde(rename = "tournamentTitle")]
    tournament_title: String,
    #[serde(rename = "tournamentSefTitle")]
    tournament_sef: String,
    #[serde(rename = "battleTag")]
    pub battle_tag: String,
    #[serde(rename = "altAccounts")]
    pub alt_accounts: Option<Vec<String>>,
    pub twitch: String,
    pub discord: String,
    #[serde(rename = "primaryRole")]
    pub primary_role: Option<RoleValue>,
    #[serde(rename = "secondaryRole")]
    pub secondary_role: Option<RoleValue>,
    pub guarantors: Option<Vec<String>>,
    #[serde(rename = "additionalInfo")]
    pub additional_info: String,
    pub status: RegistrationStatus,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,
    #[serde(rename = "managerComment")]
    pub manager_comment: Option<String>,
    #[serde(rename = "declineReason")]
    pub decline_reason: Option<String>,
    #[serde(rename = "verificationBattletag")]
    pub verification_battletag: Option<String>,
    #[serde(rename = "twitchChannel")]
    pub twitch_channel: Option<String>,
    #[serde(rename = "donationAmountRub")]
    pub donation_amount_rub: Option<f64>,
}

#[get("/{registration_id}")]
pub async fn get_registration(
    user: AuthenticatedUser,
    db: web::Data<Arc<Dal>>,
    registration_id: web::Path<i64>,
) -> actix_web::Result<impl Responder, ApiError> {
    let user_id = user.id.parse::<i64>().map_err(|_| ApiError::BadRequest {
        error: "Invalid user id".to_string(),
        details: "User id from auth token is not a number.".to_string(),
    })?;

    let registration_id = registration_id.into_inner();
    let registration = db
        .registrations
        .find_by_id_for_user(registration_id, user_id)
        .await
        .map_err(|e| {
            log::error!(
                "Failed to load registration {} for user {}: {}",
                registration_id,
                user_id,
                e
            );
            ApiError::InternalError { error: e.to_string() }
        })?
        .ok_or(ApiError::NotFound)?;

    let tournament = db
        .tournaments
        .get_by_id(registration.tournament_id)
        .await
        .map_err(|e| {
            log::error!(
                "Failed to load tournament {} for registration {}: {}",
                registration.tournament_id,
                registration_id,
                e
            );
            ApiError::InternalError { error: e.to_string() }
        })?
        .ok_or(ApiError::NotFound)?;

    let user_battle_tag = db
        .users
        .find_battletag_by_id(registration.battletag_id)
        .await
        .map_err(|e| {
            log::error!(
                "Failed to load battletag {} for registration {}: {}",
                registration.battletag_id,
                registration_id,
                e
            );
            ApiError::InternalError { error: e.to_string() }
        })?
        .ok_or(ApiError::NotFound)?;

    let eligibility = tournament.config.eligibility.as_ref();
    let verification_battletag = eligibility.and_then(|value| value.verification_battletag.clone());
    let subscription = eligibility.and_then(|value| value.subscription.as_ref());
    let twitch_channel = subscription.and_then(|value| value.twitch_channel.clone());
    let donation_amount_rub = subscription.and_then(|value| value.donation_amount_rub);

    let manager_comment = if registration.status == RegistrationStatus::ActionRequired {
        db.registrations
            .get_latest_pending_action_comment(registration_id)
            .await
            .map_err(|e| {
                log::error!(
                    "Failed to load pending action comment for registration {}: {}",
                    registration_id,
                    e
                );
                ApiError::InternalError { error: e.to_string() }
            })?
    } else {
        None
    };

    Ok(HttpResponse::Ok().json(RegistrationDetailsResponse {
        tournament_title: tournament.title,
        tournament_sef: tournament.sef_title,
        battle_tag: user_battle_tag.battletag,
        alt_accounts: registration.alt_accounts,
        twitch: registration.twitch,
        discord: registration.discord,
        primary_role: registration.primary_role,
        secondary_role: registration.secondary_role,
        guarantors: registration.guarantors,
        additional_info: registration.additional_info,
        status: registration.status,
        created_at: registration.created_at,
        updated_at: registration.updated_at,
        manager_comment,
        decline_reason: registration.decline_reason,
        verification_battletag,
        twitch_channel,
        donation_amount_rub,
    }))
}
