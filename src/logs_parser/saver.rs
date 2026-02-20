use crate::dal::{Dal, MatchRow, TeamRow};
use crate::logs_parser::event_types::{BaseLogEvent, LogEvent};

pub struct SaveContext {
    pub current_round: i64,
}

impl SaveContext {
    pub fn new() -> Self {
        Self { current_round: 0 }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("team {0} not found in tournament {1}")]
    TeamNotFound(String, String),
    #[error("hero not found: {0}")]
    HeroNotFound(String),
    #[error("match not found: {0}")]
    MatchNotFound(String),
    #[error("player not found in team \"{0}\": {1}")]
    PlayerNotFound(String, String),
}

pub async fn save_events(
    db: &Dal,
    match_row: &MatchRow,
    events: &[LogEvent],
) -> Result<(), SaveError> {
    let mut ctx = SaveContext::new();

    for event in events {
        match event {
            LogEvent::RoundStartEvent(round_start) => {
                if let Some(round) = round_start.round {
                    ctx.current_round = round;
                }
                save_single_event(db, match_row.id, event).await?;
            }
            LogEvent::PlayerStatEvent(player_stat) => {
                let round = player_stat.round.unwrap_or(ctx.current_round);
                let team = fetch_team(db, &match_row.tournament_id, &player_stat.team).await?;
                let player_id = fetch_player_id(db, &team, &player_stat.player).await?;
                let hero_id = fetch_hero_id(db, &player_stat.hero).await?;

                insert_match_player_statistics(
                    db,
                    match_row.id,
                    round,
                    player_id,
                    hero_id,
                    &player_stat.stats,
                )
                .await?;
            }
            _ => {
                save_single_event(db, match_row.id, event).await?;
            }
        }
    }

    Ok(())
}

async fn fetch_team(db: &Dal, tournament_id: &i64, team_name: &str) -> Result<TeamRow, SaveError> {
    db.teams
        .get_by_tournament_and_name(tournament_id, team_name)
        .await?
        .ok_or_else(|| SaveError::TeamNotFound(team_name.to_string(), tournament_id.to_string()))
}

async fn fetch_player_id(db: &Dal, team: &TeamRow, player: &str) -> Result<i64, SaveError> {
    if let Some(id) = db
        .players
        .find_id_by_team_and_nickname(team.id, player)
        .await?
    {
        return Ok(id);
    }

    Err(SaveError::PlayerNotFound(
        team.name.to_string(),
        player.to_string(),
    ))
}

async fn fetch_hero_id(db: &Dal, hero: &str) -> Result<i64, SaveError> {
    db.heroes
        .find_id_by_any_localized_name(hero)
        .await?
        .ok_or_else(|| SaveError::HeroNotFound(hero.to_string()))
}

async fn insert_match_player_statistics(
    db: &Dal,
    match_id: i64,
    round: i64,
    player_id: i64,
    hero_id: i64,
    stats: &[String],
) -> Result<(), SaveError> {
    db.match_player_statistics
        .insert(match_id, round, player_id, hero_id, stats)
        .await
        .map_err(|e| {
            tracing::error!(
                match_id, round, player_id, hero_id,
                stats_len = stats.len(),
                error = ?e,
                "failed inserting match_player_statistics"
            );
            e
        })?;

    Ok(())
}

async fn save_single_event(db: &Dal, match_id: i64, event: &LogEvent) -> Result<(), SaveError> {
    if matches!(event, LogEvent::PlayerStatEvent(_)) {
        return Ok(());
    }

    let time = *event.timestamp();
    let event_name = event.event_name();

    let data = serde_json::to_value(event)?;

    db.match_events
        .insert(match_id, time, event_name, data)
        .await?;

    Ok(())
}
