#[derive(Clone)]
pub struct Dal {
    pub(crate) db_pool: Pool<Sqlite>,
}

use sqlx::{
    Pool,
    Sqlite,
    SqlitePool,
};

impl Dal {
    pub async fn new(config: &crate::config::Config) -> Self {
        let sqlite_config = config.sqlite
            .clone().try_into().expect("failed to convert sqlite config");
        let db_pool = SqlitePool::connect_lazy_with(sqlite_config);
        sqlx::migrate!("src/migrations")
            .run(&db_pool)
            .await
            .expect("db initialization failed");
        Dal {
            db_pool
        }
    }

    pub async fn autovacuum(&self) -> Result<(), sqlx::Error> {
        sqlx::query("PRAGMA incremental_vacuum;")
            .execute(&self.db_pool)
            .await?;

        Ok(())
    }
}
