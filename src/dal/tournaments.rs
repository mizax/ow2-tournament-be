use serde_json;
use sqlx::{Error, Pool, Sqlite};
use thiserror::Error as ThisError;

use crate::shared_models::tournaments::models::TournamentConfig;

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
}

pub struct TournamentDetailsData {
    pub id: i64,
    pub title: String,
    pub sef_title: String,
    pub discipline: String,
    pub format: String,
    pub config: TournamentConfig,
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

impl TournamentsRepo {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn list_short(&self) -> Result<Vec<TournamentShortData>, TournamentsRepoError> {
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
                COUNT(r.id) AS registration_count
            FROM tournaments t
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
                t.prize_pool_currency
            ORDER BY t.id
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        let mut tournaments = Vec::with_capacity(rows.len());
        for row in rows {
            let dates: Vec<String> = serde_json::from_str(&row.dates_json)?;
            let prize_pool = format_prize_pool(row.prize_pool_total_amount, &row.prize_pool_currency);

            tournaments.push(TournamentShortData {
                id: row.id,
                title: row.title,
                uri: row.sef_title,
                discipline: row.discipline,
                format: row.format,
                dates,
                prize_pool,
                registration_count: row.registration_count,
            });
        }

        Ok(tournaments)
    }

    pub async fn get_id_by_sef_title(&self, sef_title: &str) -> Result<Option<i64>, Error> {
        sqlx::query_scalar::<_, i64>(
            r#"
            SELECT id FROM tournaments WHERE sef_title = ?1 LIMIT 1
            "#,
        )
            .bind(sef_title)
            .fetch_optional(&self.pool)
            .await
    }

    pub async fn get_by_sef_title(
        &self,
        sef_title: &str,
    ) -> Result<Option<TournamentDetailsData>, TournamentsRepoError> {
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
        .fetch_optional(&self.pool)
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
        .fetch_optional(&self.pool)
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
