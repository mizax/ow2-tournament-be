use sqlx::{Pool, Sqlite};

#[derive(Clone)]
pub struct PlayersRepo {
    pool: Pool<Sqlite>,
}

impl PlayersRepo {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn find_id_by_team_and_nickname(&self, team_id: i64, nickname: &str) -> Result<Option<i64>, sqlx::Error> {
        sqlx::query_scalar::<_, i64>(
            r#"SELECT id FROM players WHERE team_id = ?1 AND nickname = ?2"#,
        )
        .bind(team_id)
        .bind(nickname)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn create(&self, team_id: i64, nickname: &str) -> Result<i64, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"INSERT INTO players (team_id, nickname) VALUES (?1, ?2)"#,
        )
        .bind(team_id)
        .bind(nickname)
        .execute(&mut *tx)
        .await?;

        let id = sqlx::query_scalar::<_, i64>(r#"SELECT last_insert_rowid()"#)
            .fetch_one(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(id)
    }
}
