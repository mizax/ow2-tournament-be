use sqlx::{Executor, Pool, Sqlite};

#[derive(Clone)]
pub struct HeroesRepo {
    pool: Pool<Sqlite>,
}

impl HeroesRepo {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn find_id_by_localized_name(
        &self,
        name: &str,
        lang: &str,
    ) -> Result<Option<i64>, sqlx::Error> {
        self.find_id_by_localized_name_with_executor(&self.pool, name, lang)
            .await
    }

    pub async fn find_id_by_localized_name_with_executor<'e, E>(
        &self,
        executor: E,
        name: &str,
        lang: &str,
    ) -> Result<Option<i64>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query_scalar::<_, i64>(
            r#"SELECT hero_id FROM heroes_localization WHERE name = ?1 AND lang = ?2 LIMIT 1"#,
        )
        .bind(name)
        .bind(lang)
        .fetch_optional(executor)
        .await
    }

    pub async fn find_id_by_any_localized_name(
        &self,
        name: &str,
    ) -> Result<Option<i64>, sqlx::Error> {
        self.find_id_by_any_localized_name_with_executor(&self.pool, name)
            .await
    }

    pub async fn find_id_by_any_localized_name_with_executor<'e, E>(
        &self,
        executor: E,
        name: &str,
    ) -> Result<Option<i64>, sqlx::Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query_scalar::<_, i64>(
            r#"SELECT hero_id FROM heroes_localization WHERE name = ?1 LIMIT 1"#,
        )
        .bind(name)
        .fetch_optional(executor)
        .await
    }
}
