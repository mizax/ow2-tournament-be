#[derive(Clone)]
pub struct Dal {
    pub(crate) db_pool: Pool<Sqlite>,
    pub matches: MatchesRepo,
    pub teams: TeamsRepo,
    pub players: PlayersRepo,
    pub heroes: HeroesRepo,
    pub match_events: MatchEventsRepo,
    pub match_player_statistics: MatchPlayerStatisticsRepo,
}

use sqlx::{Pool, Sqlite, SqlitePool};

use crate::dal::{
    HeroesRepo, MatchEventsRepo, MatchPlayerStatisticsRepo, MatchesRepo, PlayersRepo, TeamsRepo,
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
            matches: MatchesRepo::new(db_pool.clone()),
            teams: TeamsRepo::new(db_pool.clone()),
            players: PlayersRepo::new(db_pool.clone()),
            heroes: HeroesRepo::new(db_pool.clone()),
            match_events: MatchEventsRepo::new(db_pool.clone()),
            match_player_statistics: MatchPlayerStatisticsRepo::new(db_pool.clone()),
            db_pool,
        }
    }

    pub async fn autovacuum(&self) -> Result<(), sqlx::Error> {
        sqlx::query("PRAGMA incremental_vacuum;")
            .execute(&self.db_pool)
            .await?;

        Ok(())
    }
}
