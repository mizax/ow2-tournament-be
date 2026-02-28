use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

pub struct TestDb {
    _dir: tempfile::TempDir,
    pub pool: SqlitePool,
}

impl TestDb {
    pub async fn new() -> Self {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("test.sqlite");
        let options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("connect sqlite");

        sqlx::migrate!("src/migrations")
            .run(&pool)
            .await
            .expect("run migrations");

        Self { _dir: dir, pool }
    }
}
