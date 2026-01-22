use sqlx::{Pool, Sqlite};

#[derive(Clone)]
pub struct PlayersRepo {
    pool: Pool<Sqlite>,
}

impl PlayersRepo {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn find_id_by_username(&self, username: &str) -> Result<Option<i64>, sqlx::Error> {
        sqlx::query_scalar::<_, i64>(
            r#"SELECT id FROM players WHERE username = ?1"#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create(&self, username: &str) -> Result<i64, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"INSERT INTO players (username) VALUES (?1)"#,
        )
        .bind(username)
        .execute(&mut *tx)
        .await?;

        let id = sqlx::query_scalar::<_, i64>(r#"SELECT last_insert_rowid()"#)
            .fetch_one(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(id)
    }
}
