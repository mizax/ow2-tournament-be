use sqlx::{Pool, Sqlite};

#[derive(Clone)]
pub struct TournamentManagersRepo {
    pool: Pool<Sqlite>,
}

impl TournamentManagersRepo {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn user_is_manager(&self, user_id: i64) -> Result<bool, sqlx::Error> {
        let exists = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT 1
            FROM tournament_managers
            WHERE user_id = ?1
            LIMIT 1
            "#,
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await?
        .is_some();

        Ok(exists)
    }
}
