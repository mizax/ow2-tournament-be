use sqlx::{Executor, Pool, Sqlite};

#[derive(Clone)]
pub struct PlayersRepo {
    pool: Pool<Sqlite>,
}

impl PlayersRepo {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn find_team_id_by_tournament_and_nickname(
        &self,
        tournament_id: i64,
        nickname: &str,
    ) -> Result<Option<i64>, sqlx::Error> {
        self.find_team_id_by_tournament_and_nickname_with_executor(
            &self.pool,
            tournament_id,
            nickname,
        )
        .await
    }

    pub async fn find_team_id_by_tournament_and_nickname_with_executor<'e, E>(
        &self,
        executor: E,
        tournament_id: i64,
        nickname: &str,
    ) -> Result<Option<i64>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT p.team_id
            FROM players p
            JOIN teams t ON t.id = p.team_id
            WHERE t.tournament_id = ?1 AND p.nickname = ?2
            LIMIT 1
            "#,
        )
        .bind(tournament_id)
        .bind(nickname)
        .fetch_optional(executor)
        .await
    }

    pub async fn find_id_by_team_and_nickname(
        &self,
        team_id: i64,
        nickname: &str,
    ) -> Result<Option<i64>, sqlx::Error> {
        self.find_id_by_team_and_nickname_with_executor(&self.pool, team_id, nickname)
            .await
    }

    pub async fn find_id_by_team_and_nickname_with_executor<'e, E>(
        &self,
        executor: E,
        team_id: i64,
        nickname: &str,
    ) -> Result<Option<i64>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query_scalar::<_, i64>(
            r#"SELECT id FROM players WHERE team_id = ?1 AND nickname = ?2"#,
        )
        .bind(team_id)
        .bind(nickname)
        .fetch_optional(executor)
        .await
    }

    pub async fn create(&self, team_id: i64, nickname: &str) -> Result<i64, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(r#"INSERT INTO players (team_id, nickname) VALUES (?1, ?2)"#)
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

    pub async fn create_with_executor<'e, E>(
        &self,
        executor: &mut E,
        team_id: i64,
        nickname: &str,
    ) -> Result<i64, sqlx::Error>
    where
        for<'c> &'c mut E: Executor<'c, Database = Sqlite>,
    {
        sqlx::query(r#"INSERT INTO players (team_id, nickname) VALUES (?1, ?2)"#)
            .bind(team_id)
            .bind(nickname)
            .execute(&mut *executor)
            .await?;

        sqlx::query_scalar::<_, i64>(r#"SELECT last_insert_rowid()"#)
            .fetch_one(&mut *executor)
            .await
    }
}
