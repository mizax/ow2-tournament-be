use sqlx::{Pool, Sqlite};
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
        .fetch_optional(&self.pool)
        .await
    }
}
