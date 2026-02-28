use serde_json;
use sqlx::{Error, Executor, Pool, Sqlite};
use thiserror::Error as ThisError;

use crate::shared_models::tournaments::models::{
    TournamentConfig, TournamentPodiumPlace, TournamentResultPlace,
};

#[derive(Clone)]
pub struct TournamentsRepo {
    pool: Pool<Sqlite>,
}

#[derive(Debug, ThisError)]
pub enum TournamentsRepoError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub struct TournamentShortData {
    pub id: i64,
    pub title: String,
    pub uri: String,
    pub discipline: String,
    pub format: String,
    pub dates: Vec<String>,
    pub prize_pool: Option<String>,
    pub registration_count: i64,
    pub podium: Option<Vec<TournamentPodiumPlace>>,
}

pub struct TournamentDetailsData {
    pub id: i64,
    pub title: String,
    pub sef_title: String,
    pub discipline: String,
    pub format: String,
    pub config: TournamentConfig,
}

pub struct TournamentIdDatesData {
    pub id: i64,
    pub dates: Vec<String>,
}

pub struct UpdateTournamentData {
    pub title: String,
    pub sef_title: String,
    pub discipline: String,
    pub format: String,
    pub dates_json: String,
    pub prize_pool_total_amount: f64,
    pub prize_pool_currency: String,
    pub configuration_json: String,
}

#[derive(sqlx::FromRow)]
struct TournamentShortRow {
    pub id: i64,
    pub title: String,
    pub sef_title: String,
    pub discipline: String,
    pub format: String,
    pub dates_json: String,
    pub prize_pool_total_amount: f64,
    pub prize_pool_currency: String,
    pub registration_count: i64,
    pub configuration_json: Option<String>,
}

#[derive(sqlx::FromRow)]
struct TournamentConfigRow {
    pub id: i64,
    pub title: String,
    pub sef_title: String,
    pub discipline: String,
    pub format: String,
    pub configuration_json: String,
}

#[derive(sqlx::FromRow)]
struct TournamentIdDatesRow {
    pub id: i64,
    pub dates_json: String,
}

impl TournamentsRepo {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn list_short(&self) -> Result<Vec<TournamentShortData>, TournamentsRepoError> {
        self.list_short_with_executor(&self.pool).await
    }

    pub async fn list_short_with_executor<'e, E>(
        &self,
        executor: E,
    ) -> Result<Vec<TournamentShortData>, TournamentsRepoError>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let rows = sqlx::query_as::<_, TournamentShortRow>(
            r#"
            SELECT
                t.id,
                t.title,
                t.sef_title,
                t.discipline,
                t.format,
                t.dates_json,
                t.prize_pool_total_amount,
                t.prize_pool_currency,
                COUNT(r.id) AS registration_count,
                tc.configuration_json
            FROM tournaments t
            LEFT JOIN tournament_configuration tc
                ON tc.tournament_id = t.id
            LEFT JOIN registrations r
                ON r.tournament_id = t.id
               AND r.status NOT IN ('DELETED', 'DECLINED')
            WHERE t.deleted_at IS NULL
            GROUP BY
                t.id,
                t.title,
                t.sef_title,
                t.discipline,
                t.format,
                t.dates_json,
                t.prize_pool_total_amount,
                t.prize_pool_currency,
                tc.configuration_json
            ORDER BY t.id
            "#,
        )
        .fetch_all(executor)
        .await?;

        let mut tournaments = Vec::with_capacity(rows.len());
        for row in rows {
            let dates: Vec<String> = serde_json::from_str(&row.dates_json)?;
            let prize_pool =
                format_prize_pool(row.prize_pool_total_amount, &row.prize_pool_currency);
            let podium = row
                .configuration_json
                .as_deref()
                .map(serde_json::from_str::<TournamentConfig>)
                .transpose()?
                .and_then(extract_podium);

            tournaments.push(TournamentShortData {
                id: row.id,
                title: row.title,
                uri: row.sef_title,
                discipline: row.discipline,
                format: row.format,
                dates,
                prize_pool,
                registration_count: row.registration_count,
                podium,
            });
        }

        Ok(tournaments)
    }

    pub async fn get_id_by_sef_title(&self, sef_title: &str) -> Result<Option<i64>, Error> {
        self.get_id_by_sef_title_with_executor(&self.pool, sef_title)
            .await
    }

    pub async fn get_id_by_sef_title_with_executor<'e, E>(
        &self,
        executor: E,
        sef_title: &str,
    ) -> Result<Option<i64>, Error>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT id FROM tournaments WHERE sef_title = ?1 LIMIT 1
            "#,
        )
        .bind(sef_title)
        .fetch_optional(executor)
        .await
    }

    pub async fn get_id_and_dates_by_sef_title(
        &self,
        sef_title: &str,
    ) -> Result<Option<TournamentIdDatesData>, TournamentsRepoError> {
        self.get_id_and_dates_by_sef_title_with_executor(&self.pool, sef_title)
            .await
    }

    pub async fn get_id_and_dates_by_sef_title_with_executor<'e, E>(
        &self,
        executor: E,
        sef_title: &str,
    ) -> Result<Option<TournamentIdDatesData>, TournamentsRepoError>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let row = sqlx::query_as::<_, TournamentIdDatesRow>(
            r#"
            SELECT
                id,
                dates_json
            FROM tournaments
            WHERE sef_title = ?1
            LIMIT 1
            "#,
        )
        .bind(sef_title)
        .fetch_optional(executor)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let dates: Vec<String> = serde_json::from_str(&row.dates_json)?;

        Ok(Some(TournamentIdDatesData { id: row.id, dates }))
    }

    pub async fn get_by_sef_title(
        &self,
        sef_title: &str,
    ) -> Result<Option<TournamentDetailsData>, TournamentsRepoError> {
        self.get_by_sef_title_with_executor(&self.pool, sef_title)
            .await
    }

    pub async fn get_by_sef_title_with_executor<'e, E>(
        &self,
        executor: E,
        sef_title: &str,
    ) -> Result<Option<TournamentDetailsData>, TournamentsRepoError>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let row = sqlx::query_as::<_, TournamentConfigRow>(
            r#"
            SELECT
                t.id,
                t.title,
                t.sef_title,
                t.discipline,
                t.format,
                tc.configuration_json
            FROM tournaments t
            INNER JOIN tournament_configuration tc
                ON tc.tournament_id = t.id
            WHERE t.sef_title = ?1
            LIMIT 1
        "#,
        )
        .bind(sef_title)
        .fetch_optional(executor)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let config: TournamentConfig = serde_json::from_str(&row.configuration_json)?;

        Ok(Some(TournamentDetailsData {
            id: row.id,
            title: row.title,
            sef_title: row.sef_title,
            discipline: row.discipline,
            format: row.format,
            config,
        }))
    }

    pub async fn get_by_id(
        &self,
        id: i64,
    ) -> Result<Option<TournamentDetailsData>, TournamentsRepoError> {
        self.get_by_id_with_executor(&self.pool, id).await
    }

    pub async fn get_by_id_with_executor<'e, E>(
        &self,
        executor: E,
        id: i64,
    ) -> Result<Option<TournamentDetailsData>, TournamentsRepoError>
    where
        E: Executor<'e, Database = Sqlite>,
    {
        let row = sqlx::query_as::<_, TournamentConfigRow>(
            r#"
            SELECT
                t.id,
                t.title,
                t.sef_title,
                t.discipline,
                t.format,
                tc.configuration_json
            FROM tournaments t
            INNER JOIN tournament_configuration tc
                ON tc.tournament_id = t.id
            WHERE t.id = ?1
            LIMIT 1
        "#,
        )
        .bind(id)
        .fetch_optional(executor)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let config: TournamentConfig = serde_json::from_str(&row.configuration_json)?;

        Ok(Some(TournamentDetailsData {
            id: row.id,
            title: row.title,
            sef_title: row.sef_title,
            discipline: row.discipline,
            format: row.format,
            config,
        }))
    }

    pub async fn update(
        &self,
        tournament_id: i64,
        data: UpdateTournamentData,
    ) -> Result<(), TournamentsRepoError> {
        let mut tx = self.pool.begin().await?;

        sqlx::query(
            r#"
            UPDATE tournaments
            SET title = ?1,
                sef_title = ?2,
                discipline = ?3,
                format = ?4,
                dates_json = ?5,
                prize_pool_total_amount = ?6,
                prize_pool_currency = ?7,
                modified_at = CURRENT_TIMESTAMP
            WHERE id = ?8
            "#,
        )
        .bind(data.title)
        .bind(data.sef_title)
        .bind(data.discipline)
        .bind(data.format)
        .bind(data.dates_json)
        .bind(data.prize_pool_total_amount)
        .bind(data.prize_pool_currency)
        .bind(tournament_id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO tournament_configuration (
                tournament_id,
                configuration_json,
                created_at,
                modified_at
            )
            VALUES (?1, ?2, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
            ON CONFLICT(tournament_id) DO UPDATE SET
                configuration_json = excluded.configuration_json,
                modified_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(tournament_id)
        .bind(data.configuration_json)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }
}

fn extract_podium(config: TournamentConfig) -> Option<Vec<TournamentPodiumPlace>> {
    let mut podium = config
        .results
        .and_then(|results| results.placements)
        .unwrap_or_default()
        .into_iter()
        .filter(is_podium_place)
        .map(|place| TournamentPodiumPlace {
            place: place.place,
            team_name: place.team_name,
        })
        .collect::<Vec<_>>();

    podium.sort_by_key(|item| item.place);

    if podium.is_empty() {
        None
    } else {
        Some(podium)
    }
}

fn is_podium_place(place: &TournamentResultPlace) -> bool {
    (1..=3).contains(&place.place)
}

fn format_prize_pool(amount: f64, currency: &str) -> Option<String> {
    if amount <= 0.0 || currency.is_empty() {
        return None;
    }

    let formatted_amount = if amount.fract() == 0.0 {
        format!("{:.0}", amount)
    } else {
        format!("{}", amount)
    };

    Some(format!("{} {}", formatted_amount, currency))
}

#[cfg(test)]
mod tests {
    use super::format_prize_pool;

    #[test]
    fn format_prize_pool_returns_none_for_invalid_inputs() {
        assert_eq!(format_prize_pool(0.0, "USD"), None);
        assert_eq!(format_prize_pool(-5.0, "USD"), None);
        assert_eq!(format_prize_pool(10.0, ""), None);
    }

    #[test]
    fn format_prize_pool_formats_integers_without_decimal() {
        assert_eq!(format_prize_pool(100.0, "USD"), Some("100 USD".to_string()));
    }

    #[test]
    fn format_prize_pool_formats_fractional_amounts() {
        assert_eq!(format_prize_pool(12.5, "EUR"), Some("12.5 EUR".to_string()));
    }
}
