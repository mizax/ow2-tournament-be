use sqlx::{Pool, Sqlite};

#[derive(Clone)]
pub struct MatchesRepo {
    pool: Pool<Sqlite>,
}

impl MatchesRepo {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn exists(&self, match_id: i64) -> Result<bool, sqlx::Error> {
        let exists = sqlx::query_scalar::<_, i64>(
            r#"SELECT 1 FROM matches WHERE id = ?1 LIMIT 1"#,
        )
        .bind(match_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(exists.is_some())
    }
}
