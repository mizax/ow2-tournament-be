use chrono::NaiveDateTime;
use sqlx::{Executor, Pool, Sqlite};

#[derive(Clone)]
pub struct MatchMapsRepo {
    pool: Pool<Sqlite>,
}

#[derive(sqlx::FromRow)]
pub struct MatchMapRow {
    pub id: i64,
    pub match_id: i64,
    pub game_map_id: i64,
    pub map_order: i64,
    pub home_score: i64,
    pub away_score: i64,
    pub log_name: Option<String>,
    pub created_at: NaiveDateTime,
    pub modified_at: NaiveDateTime,
}

impl MatchMapsRepo {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn find_game_map_id_by_alias(&self, alias: &str) -> Result<Option<i64>, sqlx::Error> {
        self.find_game_map_id_by_alias_with_executor(&self.pool, alias)
            .await
    }

    pub async fn find_game_map_id_by_alias_with_executor<'e, E>(
        &self,
        executor: E,
        alias: &str,
    ) -> Result<Option<i64>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query_scalar::<_, i64>(
            r#"SELECT game_map_id FROM map_name_aliases WHERE alias = ?1 LIMIT 1"#,
        )
        .bind(alias)
        .fetch_optional(executor)
        .await
    }

    pub async fn insert<'e, E>(
        &self,
        executor: E,
        match_id: i64,
        game_map_id: i64,
        map_order: i64,
        home_score: i64,
        away_score: i64,
        log_name: &str,
    ) -> Result<i64, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO match_maps (match_id, game_map_id, map_order, home_score, away_score, log_name)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            RETURNING id
            "#,
        )
        .bind(match_id)
        .bind(game_map_id)
        .bind(map_order)
        .bind(home_score)
        .bind(away_score)
        .bind(log_name)
        .fetch_one(executor)
        .await?;

        Ok(id)
    }

    pub async fn max_map_order_for_match(&self, match_id: i64) -> Result<i64, sqlx::Error> {
        let max = sqlx::query_scalar::<_, Option<i64>>(
            r#"SELECT MAX(map_order) FROM match_maps WHERE match_id = ?1"#,
        )
        .bind(match_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(max.unwrap_or(0))
    }

    pub async fn log_name_exists_for_match(
        &self,
        match_id: i64,
        log_name: &str,
    ) -> Result<bool, sqlx::Error> {
        let exists = sqlx::query_scalar::<_, i64>(
            r#"SELECT 1 FROM match_maps WHERE match_id = ?1 AND log_name = ?2 LIMIT 1"#,
        )
        .bind(match_id)
        .bind(log_name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(exists.is_some())
    }

    pub async fn update_scores_with_executor<'e, E>(
        &self,
        executor: E,
        id: i64,
        home_score: i64,
        away_score: i64,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query(r#"UPDATE match_maps SET home_score = ?2, away_score = ?3 WHERE id = ?1"#)
            .bind(id)
            .bind(home_score)
            .bind(away_score)
            .execute(executor)
            .await?;

        Ok(())
    }

    pub async fn get_by_match_id(&self, match_id: i64) -> Result<Vec<MatchMapRow>, sqlx::Error> {
        sqlx::query_as::<_, MatchMapRow>(
            r#"
            SELECT id, match_id, game_map_id, map_order, home_score, away_score, log_name, created_at, modified_at
            FROM match_maps
            WHERE match_id = ?1
            ORDER BY map_order
            "#,
        )
        .bind(match_id)
        .fetch_all(&self.pool)
        .await
    }
}
