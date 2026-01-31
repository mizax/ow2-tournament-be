use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, QueryBuilder, Sqlite};
use std::error::Error as StdError;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RoleValue {
    Tank,
    Damage,
    Support,
    Flex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
#[sqlx(rename_all = "SCREAMING_SNAKE_CASE")]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActionStatus {
    Pending,
    Resolved,
}

#[derive(Clone)]
pub struct RegistrationsRepo {
    pool: Pool<Sqlite>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RegistrationComment {
    pub id: i64,
    pub registration_id: i64,
    pub manager_user_id: i64,
    pub comment: String,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RegistrationRequestedAction {
    pub id: i64,
    pub registration_id: i64,
    pub manager_user_id: i64,
    pub description: String,
    pub status: ActionStatus,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RegistrationRoleRanking {
    pub id: i64,
    pub registration_id: i64,
    pub role: RoleValue,
    pub ranking: i64,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RegistrationSummary {
    pub id: i64,
    pub battletag: String,
    pub status: RegistrationStatus,
    pub primary_role: Option<RoleValue>,
    pub secondary_role: Option<RoleValue>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

pub struct RegistrationManagerDetail {
    pub registration: RegistrationRow,
    pub battletag: String,
    pub comments: Vec<RegistrationComment>,
    pub requested_actions: Vec<RegistrationRequestedAction>,
    pub role_rankings: Vec<RegistrationRoleRanking>,
}

#[derive(Debug)]
pub struct RegistrationDetails {
    pub tournament_id: i64,
    pub battletag_id: i64,
    pub alt_accounts: Option<Vec<String>>,
    pub twitch: String,
    pub discord: String,
    pub primary_role: Option<RoleValue>,
    pub secondary_role: Option<RoleValue>,
    pub guarantors: Option<Vec<String>>,
    pub additional_info: String,
    pub status: RegistrationStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub decline_reason: Option<String>,
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

#[derive(Debug, Clone, Copy)]
pub enum RegistrationSortField {
    CreatedAt,
    UpdatedAt,
}

#[derive(Debug, Clone, Copy)]
pub enum SortDirection {
    Asc,
    Desc,
}

impl RegistrationSortField {
    fn as_sql(self) -> &'static str {
        match self {
            RegistrationSortField::CreatedAt => "r.created_at",
            RegistrationSortField::UpdatedAt => "r.updated_at",
        }
    }
}

impl SortDirection {
    fn as_sql(self) -> &'static str {
        match self {
            SortDirection::Asc => "ASC",
            SortDirection::Desc => "DESC",
        }
    }
}

#[derive(sqlx::FromRow)]
struct RegistrationWithBattletagRow {
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
    pub battletag: String,
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

    pub async fn find_by_id_for_user(&self, registration_id: i64, user_id: i64) -> Result<Option<RegistrationDetails>, sqlx::Error> {
        let row = sqlx::query_as::<_, RegistrationRow>(
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
            WHERE id = ?1
              AND user_id = ?2
              AND status != ?3
            LIMIT 1
            "#)
            .bind(registration_id)
            .bind(user_id)
            .bind(RegistrationStatus::Deleted)
            .fetch_optional(&self.pool)
            .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let alt_accounts = parse_optional_list(&row.alt_accounts_json)?;
        let guarantors = parse_optional_list(&row.guarantors_json)?;

        Ok(Some(RegistrationDetails {
            tournament_id: row.tournament_id,
            battletag_id: row.user_battletag_id,
            alt_accounts,
            twitch: row.twitch,
            discord: row.discord,
            primary_role: row.primary_role,
            secondary_role: row.secondary_role,
            guarantors,
            additional_info: row.additional_info,
            status: row.status,
            created_at: DateTime::from_naive_utc_and_offset(row.created_at, Utc),
            updated_at: DateTime::from_naive_utc_and_offset(row.updated_at, Utc),
            decline_reason: row.decline_reason,
        }))
    }

    pub async fn get_latest_pending_action_comment(
        &self,
        registration_id: i64,
    ) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar::<_, String>(
            r#"
            SELECT description
            FROM registration_requested_actions
            WHERE registration_id = ?1
              AND status = ?2
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(registration_id)
        .bind(ActionStatus::Pending)
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

    pub async fn list_manager_summaries(
        &self,
        tournament_id: i64,
        statuses: Option<&[RegistrationStatus]>,
        sort_field: RegistrationSortField,
        sort_direction: SortDirection,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<RegistrationSummary>, sqlx::Error> {
        let mut builder = QueryBuilder::new(
            r#"
            SELECT
                r.id,
                ub.battletag,
                r.status,
                r.primary_role,
                r.secondary_role,
                r.created_at,
                r.updated_at
            FROM registrations r
            INNER JOIN user_battletags ub
                ON ub.id = r.user_battletag_id
            WHERE r.tournament_id = 
            "#,
        );
        builder.push_bind(tournament_id);

        if let Some(statuses) = statuses {
            if !statuses.is_empty() {
                builder.push(" AND r.status IN (");
                let mut separated = builder.separated(", ");
                for status in statuses {
                    separated.push_bind(*status);
                }
                builder.push(")");
            }
        }

        builder.push(" ORDER BY ");
        builder.push(sort_field.as_sql());
        builder.push(" ");
        builder.push(sort_direction.as_sql());
        builder.push(" LIMIT ");
        builder.push_bind(limit);
        builder.push(" OFFSET ");
        builder.push_bind(offset);

        builder
            .build_query_as::<RegistrationSummary>()
            .fetch_all(&self.pool)
            .await
    }

    pub async fn get_manager_detail(
        &self,
        registration_id: i64,
    ) -> Result<Option<RegistrationManagerDetail>, sqlx::Error> {
        let row = sqlx::query_as::<_, RegistrationWithBattletagRow>(
            r#"
            SELECT
                r.id,
                r.tournament_id,
                r.user_id,
                r.user_battletag_id,
                r.status,
                r.alt_accounts_json,
                r.twitch,
                r.discord,
                r.primary_role,
                r.secondary_role,
                r.guarantors_json,
                r.additional_info,
                r.rules_accepted,
                r.ip_address,
                r.user_agent,
                r.decline_reason,
                r.created_at,
                r.updated_at,
                r.version,
                ub.battletag
            FROM registrations r
            INNER JOIN user_battletags ub
                ON ub.id = r.user_battletag_id
            WHERE r.id = ?1
            LIMIT 1
            "#,
        )
        .bind(registration_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let registration = RegistrationRow {
            id: row.id,
            tournament_id: row.tournament_id,
            user_id: row.user_id,
            user_battletag_id: row.user_battletag_id,
            status: row.status,
            alt_accounts_json: row.alt_accounts_json,
            twitch: row.twitch,
            discord: row.discord,
            primary_role: row.primary_role,
            secondary_role: row.secondary_role,
            guarantors_json: row.guarantors_json,
            additional_info: row.additional_info,
            rules_accepted: row.rules_accepted,
            ip_address: row.ip_address,
            user_agent: row.user_agent,
            decline_reason: row.decline_reason,
            created_at: row.created_at,
            updated_at: row.updated_at,
            version: row.version,
        };

        let comments = sqlx::query_as::<_, RegistrationComment>(
            r#"
            SELECT
                id,
                registration_id,
                manager_user_id,
                comment,
                created_at
            FROM registration_comments
            WHERE registration_id = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(registration_id)
        .fetch_all(&self.pool)
        .await?;

        let requested_actions = sqlx::query_as::<_, RegistrationRequestedAction>(
            r#"
            SELECT
                id,
                registration_id,
                manager_user_id,
                description,
                status,
                created_at,
                updated_at
            FROM registration_requested_actions
            WHERE registration_id = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(registration_id)
        .fetch_all(&self.pool)
        .await?;

        let role_rankings = sqlx::query_as::<_, RegistrationRoleRanking>(
            r#"
            SELECT
                id,
                registration_id,
                role,
                ranking,
                created_at
            FROM registration_role_rankings
            WHERE registration_id = ?1
            ORDER BY role
            "#,
        )
        .bind(registration_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(Some(RegistrationManagerDetail {
            registration,
            battletag: row.battletag,
            comments,
            requested_actions,
            role_rankings,
        }))
    }

    pub async fn create_comment(
        &self,
        registration_id: i64,
        manager_user_id: i64,
        comment: String,
    ) -> Result<RegistrationComment, sqlx::Error> {
        let result = sqlx::query(
            r#"
            INSERT INTO registration_comments (
                registration_id,
                manager_user_id,
                comment,
                created_at
            )
            VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)
            "#,
        )
        .bind(registration_id)
        .bind(manager_user_id)
        .bind(comment)
        .execute(&self.pool)
        .await?;

        let comment_id = result.last_insert_rowid();
        sqlx::query_as::<_, RegistrationComment>(
            r#"
            SELECT
                id,
                registration_id,
                manager_user_id,
                comment,
                created_at
            FROM registration_comments
            WHERE id = ?1
            LIMIT 1
            "#,
        )
        .bind(comment_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn update_status_with_action(
        &self,
        registration_id: i64,
        status: RegistrationStatus,
        decline_reason: Option<String>,
        requested_action_description: Option<String>,
        manager_user_id: i64,
    ) -> Result<RegistrationRow, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            UPDATE registrations
            SET status = ?1,
                decline_reason = ?2,
                updated_at = CURRENT_TIMESTAMP,
                version = version + 1
            WHERE id = ?3
            "#,
        )
        .bind(status)
        .bind(decline_reason)
        .bind(registration_id)
        .execute(&mut *tx)
        .await?;

        if status == RegistrationStatus::ActionRequired {
            if let Some(description) = requested_action_description {
                sqlx::query(
                    r#"
                    INSERT INTO registration_requested_actions (
                        registration_id,
                        manager_user_id,
                        description,
                        status,
                        created_at,
                        updated_at
                    )
                    VALUES (?1, ?2, ?3, ?4, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
                    "#,
                )
                .bind(registration_id)
                .bind(manager_user_id)
                .bind(description)
                .bind(ActionStatus::Pending)
                .execute(&mut *tx)
                .await?;
            }
        }

        let updated = sqlx::query_as::<_, RegistrationRow>(
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
            WHERE id = ?1
            LIMIT 1
            "#,
        )
        .bind(registration_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(updated)
    }

    pub async fn replace_role_rankings(
        &self,
        registration_id: i64,
        rankings: Vec<(RoleValue, i64)>,
    ) -> Result<Vec<RegistrationRoleRanking>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            DELETE FROM registration_role_rankings
            WHERE registration_id = ?1
            "#,
        )
        .bind(registration_id)
        .execute(&mut *tx)
        .await?;

        for (role, ranking) in rankings {
            sqlx::query(
                r#"
                INSERT INTO registration_role_rankings (
                    registration_id,
                    role,
                    ranking,
                    created_at
                )
                VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)
                "#,
            )
            .bind(registration_id)
            .bind(role)
            .bind(ranking)
            .execute(&mut *tx)
            .await?;
        }

        let updated = sqlx::query_as::<_, RegistrationRoleRanking>(
            r#"
            SELECT
                id,
                registration_id,
                role,
                ranking,
                created_at
            FROM registration_role_rankings
            WHERE registration_id = ?1
            ORDER BY role
            "#,
        )
        .bind(registration_id)
        .fetch_all(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(updated)
    }

    pub async fn resolve_requested_action(
        &self,
        registration_id: i64,
        action_id: i64,
    ) -> Result<Option<RegistrationRequestedAction>, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE registration_requested_actions
            SET status = ?1,
                updated_at = CURRENT_TIMESTAMP
            WHERE id = ?2
              AND registration_id = ?3
            "#,
        )
        .bind(ActionStatus::Resolved)
        .bind(action_id)
        .bind(registration_id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Ok(None);
        }

        sqlx::query_as::<_, RegistrationRequestedAction>(
            r#"
            SELECT
                id,
                registration_id,
                manager_user_id,
                description,
                status,
                created_at,
                updated_at
            FROM registration_requested_actions
            WHERE id = ?1
              AND registration_id = ?2
            LIMIT 1
            "#,
        )
        .bind(action_id)
        .bind(registration_id)
        .fetch_optional(&self.pool)
        .await
    }
}

fn parse_optional_list(payload: &str) -> Result<Option<Vec<String>>, sqlx::Error> {
    let items: Vec<String> = serde_json::from_str(payload).map_err(|e| {
        sqlx::Error::Decode(Box::new(e) as Box<dyn StdError + Send + Sync>)
    })?;
    if items.is_empty() {
        Ok(None)
    } else {
        Ok(Some(items))
    }
}
