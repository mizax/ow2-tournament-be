use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{Executor, Pool, Sqlite};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CheckinRow {
    pub id: i64,
    pub tournament_id: i64,
    pub registration_id: i64,
    pub checked_in: bool,
    pub checked_in_at: Option<NaiveDateTime>,
    pub updated_at: NaiveDateTime,
    pub updated_by_user_id: Option<i64>,
    pub primary_role_override: Option<String>,
    pub secondary_role_override: Option<String>,
    pub role_rankings_override_json: Option<String>,
    pub full_flex_override: Option<bool>,
}

pub struct CheckinUpsert {
    pub registration_id: i64,
    pub checked_in: bool,
    pub primary_role_override: Option<String>,
    pub secondary_role_override: Option<String>,
    pub role_rankings_override_json: Option<String>,
    pub full_flex_override: Option<bool>,
}

#[derive(Clone)]
pub struct CheckinsRepo {
    pool: Pool<Sqlite>,
}

impl CheckinsRepo {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn list_for_tournament(
        &self,
        tournament_id: i64,
    ) -> Result<Vec<CheckinRow>, sqlx::Error> {
        self.list_for_tournament_with_executor(&self.pool, tournament_id)
            .await
    }

    pub async fn list_for_tournament_with_executor<'e, E>(
        &self,
        executor: E,
        tournament_id: i64,
    ) -> Result<Vec<CheckinRow>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query_as::<_, CheckinRow>(
            r#"
            SELECT
                id,
                tournament_id,
                registration_id,
                checked_in,
                checked_in_at,
                updated_at,
                updated_by_user_id,
                primary_role_override,
                secondary_role_override,
                role_rankings_override_json,
                full_flex_override
            FROM registration_checkins
            WHERE tournament_id = ?1
            ORDER BY registration_id
            "#,
        )
        .bind(tournament_id)
        .fetch_all(executor)
        .await
    }

    pub async fn get(
        &self,
        tournament_id: i64,
        registration_id: i64,
    ) -> Result<Option<CheckinRow>, sqlx::Error> {
        self.get_with_executor(&self.pool, tournament_id, registration_id)
            .await
    }

    pub async fn get_with_executor<'e, E>(
        &self,
        executor: E,
        tournament_id: i64,
        registration_id: i64,
    ) -> Result<Option<CheckinRow>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query_as::<_, CheckinRow>(
            r#"
            SELECT
                id,
                tournament_id,
                registration_id,
                checked_in,
                checked_in_at,
                updated_at,
                updated_by_user_id,
                primary_role_override,
                secondary_role_override,
                role_rankings_override_json,
                full_flex_override
            FROM registration_checkins
            WHERE tournament_id = ?1
              AND registration_id = ?2
            LIMIT 1
            "#,
        )
        .bind(tournament_id)
        .bind(registration_id)
        .fetch_optional(executor)
        .await
    }

    pub async fn upsert_single(
        &self,
        tournament_id: i64,
        user_id: i64,
        item: CheckinUpsert,
    ) -> Result<CheckinRow, sqlx::Error> {
        let mut conn = self.pool.acquire().await?;
        self.upsert_single_with_executor(&mut *conn, tournament_id, user_id, item)
            .await
    }

    pub async fn upsert_single_with_executor<'e, E>(
        &self,
        executor: &mut E,
        tournament_id: i64,
        user_id: i64,
        item: CheckinUpsert,
    ) -> Result<CheckinRow, sqlx::Error>
    where
        for<'c> &'c mut E: Executor<'c, Database = Sqlite>,
    {
        sqlx::query(
            r#"
            INSERT INTO registration_checkins (
                tournament_id,
                registration_id,
                checked_in,
                checked_in_at,
                updated_at,
                updated_by_user_id,
                primary_role_override,
                secondary_role_override,
                role_rankings_override_json,
                full_flex_override
            )
            VALUES (
                ?1, ?2, ?3,
                CASE WHEN ?3 = 1 THEN CURRENT_TIMESTAMP ELSE NULL END,
                CURRENT_TIMESTAMP,
                ?4, ?5, ?6, ?7, ?8
            )
            ON CONFLICT(registration_id) DO UPDATE SET
                checked_in = excluded.checked_in,
                checked_in_at = CASE
                    WHEN excluded.checked_in = 1 AND registration_checkins.checked_in_at IS NULL
                    THEN CURRENT_TIMESTAMP
                    WHEN excluded.checked_in = 0 THEN NULL
                    ELSE registration_checkins.checked_in_at
                END,
                updated_at = CURRENT_TIMESTAMP,
                updated_by_user_id = excluded.updated_by_user_id,
                primary_role_override = excluded.primary_role_override,
                secondary_role_override = excluded.secondary_role_override,
                role_rankings_override_json = excluded.role_rankings_override_json,
                full_flex_override = excluded.full_flex_override
            "#,
        )
        .bind(tournament_id)
        .bind(item.registration_id)
        .bind(item.checked_in)
        .bind(user_id)
        .bind(&item.primary_role_override)
        .bind(&item.secondary_role_override)
        .bind(&item.role_rankings_override_json)
        .bind(item.full_flex_override)
        .execute(&mut *executor)
        .await?;

        sqlx::query_as::<_, CheckinRow>(
            r#"
            SELECT
                id,
                tournament_id,
                registration_id,
                checked_in,
                checked_in_at,
                updated_at,
                updated_by_user_id,
                primary_role_override,
                secondary_role_override,
                role_rankings_override_json,
                full_flex_override
            FROM registration_checkins
            WHERE tournament_id = ?1
              AND registration_id = ?2
            LIMIT 1
            "#,
        )
        .bind(tournament_id)
        .bind(item.registration_id)
        .fetch_one(&mut *executor)
        .await
    }

    pub async fn upsert_batch(
        &self,
        tournament_id: i64,
        user_id: i64,
        items: Vec<CheckinUpsert>,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        for item in items {
            self.upsert_single_with_executor(&mut *tx, tournament_id, user_id, item)
                .await?;
        }
        tx.commit().await
    }

    /// Self-service: set checked_in = true for a registration (ignores overrides).
    pub async fn self_checkin(
        &self,
        tournament_id: i64,
        registration_id: i64,
        user_id: i64,
    ) -> Result<CheckinRow, sqlx::Error> {
        let mut conn = self.pool.acquire().await?;
        self.upsert_single_with_executor(
            &mut *conn,
            tournament_id,
            user_id,
            CheckinUpsert {
                registration_id,
                checked_in: true,
                primary_role_override: None,
                secondary_role_override: None,
                role_rankings_override_json: None,
                full_flex_override: None,
            },
        )
        .await
    }
}
