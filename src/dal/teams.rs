use sqlx::{Pool, Sqlite};

#[derive(Clone)]
pub struct TeamsRepo {
    pool: Pool<Sqlite>,
}

impl TeamsRepo {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn find_id_by_name(&self, name: &str) -> Result<Option<i64>, sqlx::Error> {
        sqlx::query_scalar::<_, i64>(
            r#"SELECT id FROM teams WHERE name = ?1"#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
    }
}
