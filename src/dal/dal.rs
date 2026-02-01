#[derive(Clone)]
pub struct Dal {
    pub(crate) db_pool: Pool<Sqlite>,
    pub matches: MatchesRepo,
    pub teams: TeamsRepo,
    pub players: PlayersRepo,
    pub heroes: HeroesRepo,
    pub match_events: MatchEventsRepo,
    pub match_player_statistics: MatchPlayerStatisticsRepo,
    pub users: UsersRepo,
    pub tournaments: TournamentsRepo,
    pub registrations: RegistrationsRepo,
    pub tournament_managers: TournamentManagersRepo,
}

use sqlx::{Pool, Sqlite, SqlitePool};
use futures_util::future::BoxFuture;

use crate::dal::{
    HeroesRepo, MatchEventsRepo, MatchPlayerStatisticsRepo, MatchesRepo, PlayersRepo, TeamsRepo,
    UsersRepo, TournamentsRepo, RegistrationsRepo, TournamentManagersRepo,
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
            users: UsersRepo::new(db_pool.clone()),
            tournaments: TournamentsRepo::new(db_pool.clone()),
            registrations: RegistrationsRepo::new(db_pool.clone()),
            tournament_managers: TournamentManagersRepo::new(db_pool.clone()),
            db_pool,
        }
    }

    pub fn from_pool(db_pool: Pool<Sqlite>) -> Self {
        Dal {
            matches: MatchesRepo::new(db_pool.clone()),
            teams: TeamsRepo::new(db_pool.clone()),
            players: PlayersRepo::new(db_pool.clone()),
            heroes: HeroesRepo::new(db_pool.clone()),
            match_events: MatchEventsRepo::new(db_pool.clone()),
            match_player_statistics: MatchPlayerStatisticsRepo::new(db_pool.clone()),
            users: UsersRepo::new(db_pool.clone()),
            tournaments: TournamentsRepo::new(db_pool.clone()),
            registrations: RegistrationsRepo::new(db_pool.clone()),
            tournament_managers: TournamentManagersRepo::new(db_pool.clone()),
            db_pool,
        }
    }

    pub async fn autovacuum(&self) -> Result<(), sqlx::Error> {
        sqlx::query("PRAGMA incremental_vacuum;")
            .execute(&self.db_pool)
            .await?;

        Ok(())
    }

    pub async fn transaction<F, T>(&self, f: F) -> Result<T, sqlx::Error>
    where
        F: for<'c> FnOnce(&'c Dal, &'c mut sqlx::SqliteConnection) -> BoxFuture<'c, Result<T, sqlx::Error>>,
    {
        let mut tx = self.db_pool.begin().await?;
        let result = f(self, &mut *tx).await?;
        tx.commit().await?;
        Ok(result)
    }
}
