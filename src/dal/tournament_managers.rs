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

#[derive(Debug, sqlx::FromRow)]
pub struct TournamentManagerRow {
    pub user_id: i64,
    pub battletag: Option<String>,
    pub is_owner: bool,
    pub can_manage_managers: bool,
    pub added_by_user_id: Option<i64>,
    pub added_by_battletag: Option<String>,
    pub added_at: NaiveDateTime,
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

    pub async fn add_manager_as_owner(
        &self,
        tournament_id: i64,
        user_id: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT OR IGNORE INTO tournament_managers
                (tournament_id, user_id, is_owner, can_manage_managers, added_by_user_id, added_at)
            VALUES (?1, ?2, 1, 1, NULL, CURRENT_TIMESTAMP)
            "#,
        )
        .bind(tournament_id)
        .bind(user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn add_manager_by(
        &self,
        tournament_id: i64,
        user_id: i64,
        added_by_user_id: i64,
        can_manage_managers: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO tournament_managers
                (tournament_id, user_id, is_owner, can_manage_managers, added_by_user_id, added_at)
            VALUES (?1, ?2, 0, ?3, ?4, CURRENT_TIMESTAMP)
            "#,
        )
        .bind(tournament_id)
        .bind(user_id)
        .bind(can_manage_managers)
        .bind(added_by_user_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Remove a manager, re-parenting their direct children to their parent first.
    pub async fn remove_manager(
        &self,
        tournament_id: i64,
        user_id: i64,
    ) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            UPDATE tournament_managers
            SET added_by_user_id = (
                SELECT added_by_user_id
                FROM tournament_managers
                WHERE tournament_id = ?1 AND user_id = ?2
            )
            WHERE tournament_id = ?1 AND added_by_user_id = ?2
            "#,
        )
        .bind(tournament_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            DELETE FROM tournament_managers
            WHERE tournament_id = ?1 AND user_id = ?2
            "#,
        )
        .bind(tournament_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn list_managers(
        &self,
        tournament_id: i64,
    ) -> Result<Vec<TournamentManagerRow>, sqlx::Error> {
        sqlx::query_as::<_, TournamentManagerRow>(
            r#"
            SELECT
                tm.user_id,
                ub.battletag,
                tm.is_owner,
                tm.can_manage_managers,
                tm.added_by_user_id,
                added_by_ub.battletag AS added_by_battletag,
                tm.added_at
            FROM tournament_managers tm
            LEFT JOIN user_battletags ub ON ub.id = (
                SELECT ub2.id FROM user_battletags ub2
                WHERE ub2.user_id = tm.user_id
                ORDER BY ub2.id DESC LIMIT 1
            )
            LEFT JOIN user_battletags added_by_ub ON added_by_ub.id = (
                SELECT ub3.id FROM user_battletags ub3
                WHERE ub3.user_id = tm.added_by_user_id
                ORDER BY ub3.id DESC LIMIT 1
            )
            WHERE tm.tournament_id = ?1
            ORDER BY tm.added_at ASC
            "#,
        )
        .bind(tournament_id)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn user_can_manage_managers(
        &self,
        tournament_id: i64,
        user_id: i64,
    ) -> Result<bool, sqlx::Error> {
        let can = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT can_manage_managers
            FROM tournament_managers
            WHERE tournament_id = ?1 AND user_id = ?2
            LIMIT 1
            "#,
        )
        .bind(tournament_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .map(|v| v != 0)
        .unwrap_or(false);

        Ok(can)
    }

    /// Returns true if caller_id is an ancestor of target_id in the manager tree.
    pub async fn caller_is_ancestor_of(
        &self,
        tournament_id: i64,
        caller_id: i64,
        target_id: i64,
    ) -> Result<bool, sqlx::Error> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            WITH RECURSIVE ancestors(current_user_id) AS (
                SELECT added_by_user_id
                FROM tournament_managers
                WHERE tournament_id = ?1 AND user_id = ?3 AND added_by_user_id IS NOT NULL
                UNION ALL
                SELECT tm.added_by_user_id
                FROM tournament_managers tm
                INNER JOIN ancestors a ON tm.user_id = a.current_user_id
                WHERE tm.tournament_id = ?1 AND tm.added_by_user_id IS NOT NULL
            )
            SELECT COUNT(*) FROM ancestors WHERE current_user_id = ?2
            "#,
        )
        .bind(tournament_id)
        .bind(caller_id)
        .bind(target_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(count > 0)
    }

    /// Returns user_ids of all descendants of caller_id in the manager tree.
    pub async fn get_descendants(
        &self,
        tournament_id: i64,
        caller_id: i64,
    ) -> Result<Vec<i64>, sqlx::Error> {
        sqlx::query_scalar::<_, i64>(
            r#"
            WITH RECURSIVE subtree(user_id) AS (
                SELECT user_id
                FROM tournament_managers
                WHERE tournament_id = ?1 AND added_by_user_id = ?2
                UNION ALL
                SELECT tm.user_id
                FROM tournament_managers tm
                INNER JOIN subtree s ON tm.added_by_user_id = s.user_id
                WHERE tm.tournament_id = ?1
            )
            SELECT user_id FROM subtree
            "#,
        )
        .bind(tournament_id)
        .bind(caller_id)
        .fetch_all(&self.pool)
        .await
    }
}
