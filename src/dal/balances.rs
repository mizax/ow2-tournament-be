use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::{Executor, Pool, Sqlite};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BalanceMeta {
    pub id: i64,
    pub tournament_id: i64,
    pub created_by: i64,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BalanceRow {
    pub id: i64,
    pub tournament_id: i64,
    pub created_by: i64,
    pub payload_json: String,
    pub created_at: NaiveDateTime,
}

#[derive(Clone)]
pub struct BalancesRepo {
    pool: Pool<Sqlite>,
}

impl BalancesRepo {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn save(
        &self,
        tournament_id: i64,
        user_id: i64,
        payload: serde_json::Value,
    ) -> Result<i64, sqlx::Error> {
        self.save_with_executor(&self.pool, tournament_id, user_id, payload)
            .await
    }

    pub async fn save_with_executor<'e, E>(
        &self,
        executor: E,
        tournament_id: i64,
        user_id: i64,
        payload: serde_json::Value,
    ) -> Result<i64, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let payload_str = payload.to_string();
        let result = sqlx::query(
            r#"
            INSERT INTO tournament_balances (
                tournament_id,
                created_by,
                payload_json,
                created_at
            )
            VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP)
            "#,
        )
        .bind(tournament_id)
        .bind(user_id)
        .bind(payload_str)
        .execute(executor)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn list_for_tournament(
        &self,
        tournament_id: i64,
    ) -> Result<Vec<BalanceMeta>, sqlx::Error> {
        self.list_for_tournament_with_executor(&self.pool, tournament_id)
            .await
    }

    pub async fn list_for_tournament_with_executor<'e, E>(
        &self,
        executor: E,
        tournament_id: i64,
    ) -> Result<Vec<BalanceMeta>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query_as::<_, BalanceMeta>(
            r#"
            SELECT id, tournament_id, created_by, created_at
            FROM tournament_balances
            WHERE tournament_id = ?1
            ORDER BY created_at DESC
            "#,
        )
        .bind(tournament_id)
        .fetch_all(executor)
        .await
    }

    pub async fn get(&self, balance_id: i64) -> Result<Option<BalanceRow>, sqlx::Error> {
        self.get_with_executor(&self.pool, balance_id).await
    }

    pub async fn get_with_executor<'e, E>(
        &self,
        executor: E,
        balance_id: i64,
    ) -> Result<Option<BalanceRow>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query_as::<_, BalanceRow>(
            r#"
            SELECT id, tournament_id, created_by, payload_json, created_at
            FROM tournament_balances
            WHERE id = ?1
            LIMIT 1
            "#,
        )
        .bind(balance_id)
        .fetch_optional(executor)
        .await
    }
}
