use sqlx::{Executor, Pool, Sqlite};
use chrono::NaiveDateTime;

#[derive(Clone)]
pub struct TeamsRepo {
    pool: Pool<Sqlite>,
}

#[derive(sqlx::FromRow)]
pub struct TeamRow
{
    pub id: i64,
    pub tournament_id: i64,
    pub name: String,
    pub created_at: NaiveDateTime,
    pub modified_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,
}

impl TeamsRepo {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn get_by_tournament_and_name(&self, tournament_id: &i64, name: &str) -> Result<Option<TeamRow>, sqlx::Error> {
        self.get_by_tournament_and_name_with_executor(&self.pool, tournament_id, name)
            .await
    }

    pub async fn get_by_tournament_and_name_with_executor<'e, E>(
        &self,
        executor: E,
        tournament_id: &i64,
        name: &str,
    ) -> Result<Option<TeamRow>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query_as::<_, TeamRow>(
            r#"
            SELECT
                id,
                tournament_id,
                name,
                created_at,
                modified_at,
                deleted_at
            FROM teams
            WHERE tournament_id = ?1 AND name = ?2
            LIMIT 1
            "#,
        )
        .bind(tournament_id)
        .bind(name)
        .fetch_optional(executor)
        .await
    }
}
