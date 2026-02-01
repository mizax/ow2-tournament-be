use chrono::NaiveTime;
use serde_json::Value;
use sqlx::{Executor, Pool, Sqlite};

#[derive(Clone)]
pub struct MatchEventsRepo {
    pool: Pool<Sqlite>,
}

impl MatchEventsRepo {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn insert(
        &self,
        match_id: i64,
        time: NaiveTime,
        event_name: &str,
        data: Value,
    ) -> Result<(), sqlx::Error> {
        self.insert_with_executor(&self.pool, match_id, time, event_name, data)
            .await
    }

    pub async fn insert_with_executor<'e, E>(
        &self,
        executor: E,
        match_id: i64,
        time: NaiveTime,
        event_name: &str,
        data: Value,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query(
            r#"
            INSERT INTO match_events (
              match_id, time, event, data
            ) VALUES (
              ?1, ?2, ?3, ?4
            )
            "#,
        )
        .bind(match_id)
        .bind(time)
        .bind(event_name)
        .bind(data)
        .execute(executor)
        .await?;

        Ok(())
    }
}
