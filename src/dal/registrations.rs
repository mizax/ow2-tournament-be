use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RegistrationStatus {
    Pending,
    Processing,
    Accepted,
    ActionRequired,
    Declined,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RoleValue {
    Tank,
    Damage,
    Support,
    Flex,
}

#[derive(Clone)]
pub struct RegistrationsRepo {
    pool: Pool<Sqlite>,
}

#[derive(sqlx::FromRow)]
pub struct RegistrationRow {
    pub id: i64,
    pub tournament_id: i64,
    pub user_id: i64,
    pub user_battletag_id: i64,
    pub status: RegistrationStatus,
    pub alt_accounts_json: String,
    pub twitch: String,
    pub discord: String,
    pub primary_role: Option<RoleValue>,
    pub secondary_role: Option<RoleValue>,
    pub guarantors_json: String,
    pub additional_info: String,
    pub rules_accepted: bool,
    pub ip_address: String,
    pub user_agent: String,
    pub decline_reason: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub version: i64,
}

pub struct NewRegistration {
    pub tournament_id: i64,
    pub user_id: i64,
    pub user_battletag_id: i64,
    pub status: RegistrationStatus,
    pub alt_accounts_json: String,
    pub twitch: String,
    pub discord: String,
    pub primary_role: Option<RoleValue>,
    pub secondary_role: Option<RoleValue>,
    pub guarantors_json: String,
    pub additional_info: String,
    pub rules_accepted: bool,
    pub ip_address: String,
    pub user_agent: String,
    pub decline_reason: Option<String>,
}

impl RegistrationsRepo {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn find_user_registration(&self, user_id: i64, tournament_id: i64) -> Result<Option<RegistrationRow>, sqlx::Error> {
        sqlx::query_as::<_, RegistrationRow>(
            r#"
            SELECT
                id,
                tournament_id,
                user_id,
                user_battletag_id,
                status,
                alt_accounts_json,
                twitch,
                discord,
                primary_role,
                secondary_role,
                guarantors_json,
                additional_info,
                rules_accepted,
                ip_address,
                user_agent,
                decline_reason,
                created_at,
                updated_at,
                version
            FROM registrations
            WHERE tournament_id = ?1
              AND status != ?2
              AND user_id = ?3
            ORDER BY created_at DESC
            LIMIT 1
            "#)
            .bind(tournament_id)
            .bind(RegistrationStatus::Deleted)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn active_registration_exists(
        &self,
        tournament_id: i64,
        user_id: i64,
    ) -> Result<bool, sqlx::Error> {
        let exists = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT 1
            FROM registrations
            WHERE tournament_id = ?1
              AND user_id = ?2
              AND status != ?3
            LIMIT 1
            "#,
        )
            .bind(tournament_id)
            .bind(user_id)
            .bind(RegistrationStatus::Deleted)
            .fetch_optional(&self.pool)
            .await?
            .is_some();

        Ok(exists)
    }

    pub async fn create(&self, registration: NewRegistration) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            INSERT INTO registrations (
                tournament_id,
                user_id,
                user_battletag_id,
                status,
                alt_accounts_json,
                twitch,
                discord,
                primary_role,
                secondary_role,
                guarantors_json,
                additional_info,
                rules_accepted,
                ip_address,
                user_agent,
                decline_reason,
                created_at,
                updated_at,
                version
            )
            VALUES (
                ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                CURRENT_TIMESTAMP,
                CURRENT_TIMESTAMP,
                1
            )
            "#,
        )
            .bind(registration.tournament_id)
            .bind(registration.user_id)
            .bind(registration.user_battletag_id)
            .bind(registration.status)
            .bind(registration.alt_accounts_json)
            .bind(registration.twitch)
            .bind(registration.discord)
            .bind(registration.primary_role)
            .bind(registration.secondary_role)
            .bind(registration.guarantors_json)
            .bind(registration.additional_info)
            .bind(registration.rules_accepted)
            .bind(registration.ip_address)
            .bind(registration.user_agent)
            .bind(registration.decline_reason)
            .execute(&self.pool)
            .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn list_accepted_for_tournament(
        &self,
        tournament_id: i64,
    ) -> Result<Vec<RegistrationRow>, sqlx::Error> {
        sqlx::query_as::<_, RegistrationRow>(
            r#"
            SELECT
                id,
                tournament_id,
                user_id,
                user_battletag_id,
                status,
                alt_accounts_json,
                twitch,
                discord,
                primary_role,
                secondary_role,
                guarantors_json,
                additional_info,
                rules_accepted,
                ip_address,
                user_agent,
                decline_reason,
                created_at,
                updated_at,
                version
            FROM registrations
            WHERE tournament_id = ?1
              AND status = ?2
            ORDER BY id
            "#,
        )
            .bind(tournament_id)
            .bind(RegistrationStatus::Accepted)
            .fetch_all(&self.pool)
            .await
    }
}
