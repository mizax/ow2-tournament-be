use chrono::NaiveDateTime;
use sqlx::{Executor, Pool, Sqlite};

use crate::shared_models::tournaments::models::TournamentStatus;

#[derive(Clone)]
pub struct TournamentManagersRepo {
    pool: Pool<Sqlite>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct ManagedTournamentRow {
    pub id: i64,
    pub title: String,
    pub sef_title: String,
    pub status: TournamentStatus,
    pub started_at: Option<NaiveDateTime>,
    pub registration_count: i64,
}

impl TournamentManagersRepo {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn user_is_manager(&self, user_id: i64) -> Result<bool, sqlx::Error> {
        self.user_is_manager_with_executor(&self.pool, user_id)
            .await
    }

    pub async fn user_is_manager_with_executor<'e, E>(
        &self,
        executor: E,
        user_id: i64,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let exists = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT 1
            FROM tournament_managers
            WHERE user_id = ?1
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(executor)
        .await?
        .is_some();

        Ok(exists)
    }

    pub async fn user_is_manager_for_tournament(
        &self,
        tournament_id: i64,
        user_id: i64,
    ) -> Result<bool, sqlx::Error> {
        self.user_is_manager_for_tournament_with_executor(&self.pool, tournament_id, user_id)
            .await
    }

    pub async fn user_is_manager_for_tournament_with_executor<'e, E>(
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
            FROM tournament_managers
            WHERE tournament_id = ?1
              AND user_id = ?2
            LIMIT 1
            "#,
        )
        .bind(tournament_id)
        .bind(user_id)
        .fetch_optional(executor)
        .await?
        .is_some();

        Ok(exists)
    }

    pub async fn user_is_manager_for_registration(
        &self,
        registration_id: i64,
        user_id: i64,
    ) -> Result<bool, sqlx::Error> {
        self.user_is_manager_for_registration_with_executor(&self.pool, registration_id, user_id)
            .await
    }

    pub async fn user_is_manager_for_registration_with_executor<'e, E>(
        &self,
        executor: E,
        registration_id: i64,
        user_id: i64,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let exists = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT 1
            FROM registrations r
            INNER JOIN tournament_managers tm
                ON tm.tournament_id = r.tournament_id
            WHERE r.id = ?1
              AND tm.user_id = ?2
            LIMIT 1
            "#,
        )
        .bind(registration_id)
        .bind(user_id)
        .fetch_optional(executor)
        .await?
        .is_some();

        Ok(exists)
    }

    pub async fn list_managed_tournaments(
        &self,
        user_id: i64,
    ) -> Result<Vec<ManagedTournamentRow>, sqlx::Error> {
        self.list_managed_tournaments_with_executor(&self.pool, user_id)
            .await
    }

    pub async fn list_managed_tournaments_with_executor<'e, E>(
        &self,
        executor: E,
        user_id: i64,
    ) -> Result<Vec<ManagedTournamentRow>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query_as::<_, ManagedTournamentRow>(
            r#"
            SELECT
                t.id,
                t.title,
                t.sef_title,
                t.status,
                t.started_at,
                COUNT(r.id) AS registration_count
            FROM tournaments t
            INNER JOIN tournament_managers tm
                ON tm.tournament_id = t.id
            LEFT JOIN registrations r
                ON r.tournament_id = t.id
               AND r.status != 'DELETED'
            WHERE tm.user_id = ?1
              AND t.deleted_at IS NULL
            GROUP BY t.id, t.title, t.sef_title, t.status, t.started_at
            ORDER BY t.id
            "#,
        )
        .bind(user_id)
        .fetch_all(executor)
        .await
    }

    pub async fn add_manager(&self, tournament_id: i64, user_id: i64) -> Result<(), sqlx::Error> {
        self.add_manager_with_executor(&self.pool, tournament_id, user_id)
            .await
    }

    pub async fn add_manager_with_executor<'e, E>(
        &self,
        executor: E,
        tournament_id: i64,
        user_id: i64,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO tournament_managers (tournament_id, user_id)
            VALUES (?1, ?2)
            "#,
        )
        .bind(tournament_id)
        .bind(user_id)
        .execute(executor)
        .await?;

        Ok(())
    }
}
