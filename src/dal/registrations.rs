use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{Executor, Pool, QueryBuilder, Sqlite};
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

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PublicRegistrationSummary {
    pub battletag: String,
    pub primary_role: Option<RoleValue>,
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
        self.find_user_registration_with_executor(&self.pool, user_id, tournament_id)
            .await
    }

    pub async fn find_user_registration_with_executor<'e, E>(
        &self,
        executor: E,
        user_id: i64,
        tournament_id: i64,
    ) -> Result<Option<RegistrationRow>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
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
            .fetch_optional(executor)
            .await
    }

    pub async fn find_by_id_for_user(&self, registration_id: i64, user_id: i64) -> Result<Option<RegistrationDetails>, sqlx::Error> {
        self.find_by_id_for_user_with_executor(&self.pool, registration_id, user_id)
            .await
    }

    pub async fn find_by_id_for_user_with_executor<'e, E>(
        &self,
        executor: E,
        registration_id: i64,
        user_id: i64,
    ) -> Result<Option<RegistrationDetails>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
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
            .fetch_optional(executor)
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
        self.get_latest_pending_action_comment_with_executor(&self.pool, registration_id)
            .await
    }

    pub async fn get_latest_pending_action_comment_with_executor<'e, E>(
        &self,
        executor: E,
        registration_id: i64,
    ) -> Result<Option<String>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
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
        .fetch_optional(executor)
        .await
    }

    pub async fn active_registration_exists(
        &self,
        tournament_id: i64,
        user_id: i64,
    ) -> Result<bool, sqlx::Error> {
        self.active_registration_exists_with_executor(&self.pool, tournament_id, user_id)
            .await
    }

    pub async fn active_registration_exists_with_executor<'e, E>(
        &self,
        executor: E,
        tournament_id: i64,
        user_id: i64,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
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
            .fetch_optional(executor)
            .await?
            .is_some();

        Ok(exists)
    }

    pub async fn create(&self, registration: NewRegistration) -> Result<i64, sqlx::Error> {
        self.create_with_executor(&self.pool, registration).await
    }

    pub async fn create_with_executor<'e, E>(
        &self,
        executor: E,
        registration: NewRegistration,
    ) -> Result<i64, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
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
            .execute(executor)
            .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn list_accepted_for_tournament(
        &self,
        tournament_id: i64,
    ) -> Result<Vec<RegistrationRow>, sqlx::Error> {
        self.list_accepted_for_tournament_with_executor(&self.pool, tournament_id)
            .await
    }

    pub async fn list_accepted_for_tournament_with_executor<'e, E>(
        &self,
        executor: E,
        tournament_id: i64,
    ) -> Result<Vec<RegistrationRow>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
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
            .fetch_all(executor)
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
        battletag_pattern: Option<String>,
    ) -> Result<Vec<RegistrationSummary>, sqlx::Error> {
        self.list_manager_summaries_with_executor(
            &self.pool,
            tournament_id,
            statuses,
            sort_field,
            sort_direction,
            offset,
            limit,
            battletag_pattern,
        )
        .await
    }

    pub async fn list_manager_summaries_with_executor<'e, E>(
        &self,
        executor: E,
        tournament_id: i64,
        statuses: Option<&[RegistrationStatus]>,
        sort_field: RegistrationSortField,
        sort_direction: SortDirection,
        offset: i64,
        limit: i64,
        battletag_pattern: Option<String>,
    ) -> Result<Vec<RegistrationSummary>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
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

        if let Some(pattern) = battletag_pattern {
            builder.push(" AND text_lower(ub.battletag) LIKE ");
            builder.push_bind(pattern);
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
            .fetch_all(executor)
            .await
    }

    pub async fn list_public_summaries(
        &self,
        tournament_id: i64,
    ) -> Result<Vec<PublicRegistrationSummary>, sqlx::Error> {
        self.list_public_summaries_with_executor(&self.pool, tournament_id)
            .await
    }

    pub async fn list_public_summaries_with_executor<'e, E>(
        &self,
        executor: E,
        tournament_id: i64,
    ) -> Result<Vec<PublicRegistrationSummary>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query_as::<_, PublicRegistrationSummary>(
            r#"
            SELECT
                ub.battletag,
                r.primary_role
            FROM registrations r
            INNER JOIN user_battletags ub
                ON ub.id = r.user_battletag_id
            WHERE r.tournament_id = ?1
              AND r.status NOT IN (?2, ?3)
            ORDER BY text_lower(ub.battletag)
            "#,
        )
            .bind(tournament_id)
            .bind(RegistrationStatus::Declined)
            .bind(RegistrationStatus::Deleted)
            .fetch_all(executor)
            .await
    }

    pub async fn count_manager_summaries(
        &self,
        tournament_id: i64,
        statuses: Option<&[RegistrationStatus]>,
        battletag_pattern: Option<String>,
    ) -> Result<i64, sqlx::Error> {
        self.count_manager_summaries_with_executor(&self.pool, tournament_id, statuses, battletag_pattern)
            .await
    }

    pub async fn count_manager_summaries_with_executor<'e, E>(
        &self,
        executor: E,
        tournament_id: i64,
        statuses: Option<&[RegistrationStatus]>,
        battletag_pattern: Option<String>,
    ) -> Result<i64, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let mut builder = QueryBuilder::new(
            r#"
            SELECT COUNT(*)
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

        if let Some(pattern) = battletag_pattern {
            builder.push(" AND text_lower(ub.battletag) LIKE ");
            builder.push_bind(pattern);
        }

        let row = builder
            .build_query_as::<(i64,)>()
            .fetch_one(executor)
            .await?;

        Ok(row.0)
    }

    pub async fn get_manager_detail(
        &self,
        registration_id: i64,
    ) -> Result<Option<RegistrationManagerDetail>, sqlx::Error> {
        let mut conn = self.pool.acquire().await?;
        self.get_manager_detail_with_executor(&mut *conn, registration_id)
            .await
    }

    pub async fn get_manager_detail_with_executor<'e, E>(
        &self,
        executor: &mut E,
        registration_id: i64,
    ) -> Result<Option<RegistrationManagerDetail>, sqlx::Error>
    where
        for<'c> &'c mut E: Executor<'c, Database = Sqlite>,
    {
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
        .fetch_optional(&mut *executor)
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
        .fetch_all(&mut *executor)
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
        .fetch_all(&mut *executor)
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
        .fetch_all(&mut *executor)
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
        let mut conn = self.pool.acquire().await?;
        self.create_comment_with_executor(&mut *conn, registration_id, manager_user_id, comment)
            .await
    }

    pub async fn create_comment_with_executor<'e, E>(
        &self,
        executor: &mut E,
        registration_id: i64,
        manager_user_id: i64,
        comment: String,
    ) -> Result<RegistrationComment, sqlx::Error>
    where
        for<'c> &'c mut E: Executor<'c, Database = Sqlite>,
    {
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
        .execute(&mut *executor)
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
        .fetch_one(&mut *executor)
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

        let updated = self
            .update_status_with_action_with_executor(
                &mut *tx,
                registration_id,
                status,
                decline_reason,
                requested_action_description,
                manager_user_id,
            )
            .await?;

        tx.commit().await?;
        Ok(updated)
    }

    pub async fn update_status_with_action_with_executor<'e, E>(
        &self,
        executor: &mut E,
        registration_id: i64,
        status: RegistrationStatus,
        decline_reason: Option<String>,
        requested_action_description: Option<String>,
        manager_user_id: i64,
    ) -> Result<RegistrationRow, sqlx::Error>
    where
        for<'c> &'c mut E: Executor<'c, Database = Sqlite>,
    {
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
        .execute(&mut *executor)
        .await?;

        if status == RegistrationStatus::ActionRequired {
            if let Some(description) = requested_action_description {
                let updated = sqlx::query(
                    r#"
                    UPDATE registration_requested_actions
                    SET description = ?1,
                        manager_user_id = ?2,
                        updated_at = CURRENT_TIMESTAMP
                    WHERE id = (
                        SELECT id
                        FROM registration_requested_actions
                        WHERE registration_id = ?3
                          AND status = ?4
                        ORDER BY created_at DESC
                        LIMIT 1
                    )
                    "#,
                )
                .bind(&description)
                .bind(manager_user_id)
                .bind(registration_id)
                .bind(ActionStatus::Pending)
                .execute(&mut *executor)
                .await?;

                if updated.rows_affected() == 0 {
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
                    .bind(&description)
                    .bind(ActionStatus::Pending)
                    .execute(&mut *executor)
                    .await?;
                }
            }
        } else {
            sqlx::query(
                r#"
                UPDATE registration_requested_actions
                SET status = ?1,
                    updated_at = CURRENT_TIMESTAMP
                WHERE registration_id = ?2
                  AND status = ?3
                "#,
            )
            .bind(ActionStatus::Resolved)
            .bind(registration_id)
            .bind(ActionStatus::Pending)
            .execute(&mut *executor)
            .await?;
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
        .fetch_one(&mut *executor)
        .await?;

        Ok(updated)
    }

    pub async fn replace_role_rankings(
        &self,
        registration_id: i64,
        rankings: Vec<(RoleValue, i64)>,
    ) -> Result<Vec<RegistrationRoleRanking>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let updated = self
            .replace_role_rankings_with_executor(&mut *tx, registration_id, rankings)
            .await?;

        tx.commit().await?;
        Ok(updated)
    }

    pub async fn replace_role_rankings_with_executor<'e, E>(
        &self,
        executor: &mut E,
        registration_id: i64,
        rankings: Vec<(RoleValue, i64)>,
    ) -> Result<Vec<RegistrationRoleRanking>, sqlx::Error>
    where
        for<'c> &'c mut E: Executor<'c, Database = Sqlite>,
    {
        sqlx::query(
            r#"
            DELETE FROM registration_role_rankings
            WHERE registration_id = ?1
            "#,
        )
        .bind(registration_id)
        .execute(&mut *executor)
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
            .execute(&mut *executor)
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
        .fetch_all(&mut *executor)
        .await?;

        Ok(updated)
    }

    pub async fn resolve_requested_action(
        &self,
        registration_id: i64,
        action_id: i64,
    ) -> Result<Option<RegistrationRequestedAction>, sqlx::Error> {
        let mut conn = self.pool.acquire().await?;
        self.resolve_requested_action_with_executor(&mut *conn, registration_id, action_id)
            .await
    }

    pub async fn resolve_requested_action_with_executor<'e, E>(
        &self,
        executor: &mut E,
        registration_id: i64,
        action_id: i64,
    ) -> Result<Option<RegistrationRequestedAction>, sqlx::Error>
    where
        for<'c> &'c mut E: Executor<'c, Database = Sqlite>,
    {
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
        .execute(&mut *executor)
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
        .fetch_optional(&mut *executor)
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

#[cfg(test)]
mod tests {
    use super::parse_optional_list;

    #[test]
    fn parse_optional_list_returns_none_for_empty_list() {
        let result = parse_optional_list("[]").expect("should parse");
        assert_eq!(result, None);
    }

    #[test]
    fn parse_optional_list_returns_values_for_non_empty_list() {
        let result = parse_optional_list("[\"a\",\"b\"]").expect("should parse");
        assert_eq!(result, Some(vec!["a".to_string(), "b".to_string()]));
    }

    #[test]
    fn parse_optional_list_returns_error_for_invalid_json() {
        let result = parse_optional_list("{not-json}");
        assert!(result.is_err());
    }
}
