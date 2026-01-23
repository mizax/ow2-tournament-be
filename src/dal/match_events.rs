use chrono::NaiveTime;
use serde_json::Value;
use sqlx::{Pool, Sqlite};

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
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
