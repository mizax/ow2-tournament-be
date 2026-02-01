use sqlx::{Executor, Pool, Sqlite};
use chrono::NaiveDateTime;

#[derive(Clone)]
pub struct MatchesRepo {
    pool: Pool<Sqlite>,
}

#[derive(sqlx::FromRow)]
pub struct MatchRow {
    pub id: i64,
    pub tournament_id: i64,
    pub home_team_id: i64,
    pub away_team_id: i64,
    pub home_score: i64,
    pub away_score: i64,
    pub duration: f32,
    pub log_name: String,
    pub created_at: NaiveDateTime,
    pub modified_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,
}

impl MatchesRepo {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn exists(&self, match_id: i64) -> Result<bool, sqlx::Error> {
        self.exists_with_executor(&self.pool, match_id).await
    }

    pub async fn exists_with_executor<'e, E>(
        &self,
        executor: E,
        match_id: i64,
    ) -> Result<bool, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let exists = sqlx::query_scalar::<_, i64>(
            r#"SELECT 1 FROM matches WHERE id = ?1 LIMIT 1"#,
        )
        .bind(match_id)
        .fetch_optional(executor)
        .await?;

        Ok(exists.is_some())
    }

    pub async fn get_by_id(&self, match_id: i64) -> Result<Option<MatchRow>, sqlx::Error> {
        self.get_by_id_with_executor(&self.pool, match_id).await
    }

    pub async fn get_by_id_with_executor<'e, E>(
        &self,
        executor: E,
        match_id: i64,
    ) -> Result<Option<MatchRow>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query_as::<_, MatchRow>(
            r#"
            SELECT
              id,
              tournament_id,
              home_team_id,
              away_team_id,
              home_score,
              away_score,
              duration,
              log_name,
              created_at,
              modified_at,
              deleted_at
            FROM matches
            WHERE id = ?1
            LIMIT 1
        "#,
        )
        .bind(match_id)
        .fetch_optional(executor)
        .await
    }
}
