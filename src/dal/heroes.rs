use sqlx::{Pool, Sqlite};

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
        sqlx::query_scalar::<_, i64>(
            r#"SELECT hero_id FROM heroes_localization WHERE name = ?1 AND lang = ?2 LIMIT 1"#,
        )
        .bind(name)
        .bind(lang)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn find_id_by_any_localized_name(
        &self,
        name: &str,
    ) -> Result<Option<i64>, sqlx::Error> {
        sqlx::query_scalar::<_, i64>(
            r#"SELECT hero_id FROM heroes_localization WHERE name = ?1 LIMIT 1"#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
    }
}
