use std::collections::HashMap;

use sqlx::SqliteConnection;

use crate::dal::{Dal, MatchRow, TeamRow};
use crate::logs_parser::event_types::{BaseLogEvent, LogEvent};

pub struct SaveContext {
    pub current_round: i64,
    pub current_match_map_id: Option<i64>,
    pub current_map_order: i64,
    /// Maps workshop team label (e.g. "Варрканчик") to the real team_id in the DB.
    pub team_label_to_id: HashMap<String, i64>,
    /// Team labels from the most recent match_start event (team1, team2 in log order).
    pub current_team1_label: Option<String>,
    pub current_team2_label: Option<String>,
}

impl SaveContext {
    pub fn new() -> Self {
        Self {
            current_round: 0,
            current_match_map_id: None,
            current_map_order: 0,
            team_label_to_id: HashMap::new(),
            current_team1_label: None,
            current_team2_label: None,
        }
    }
}

const SUPPORTED_LOG_VERSION: &str = "v20241022";

#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("log \"{0}\" already loaded for match {1}")]
    DuplicateLog(String, i64),
    #[error("unsupported log version: \"{0}\" (expected \"{1}\")")]
    UnsupportedLogVersion(String, String),
    #[error("team {0} not found in tournament {1}")]
    TeamNotFound(String, String),
    #[error("hero not found: {0}")]
    HeroNotFound(String),
    #[error("match not found: {0}")]
    MatchNotFound(String),
    #[error("player not found in team \"{0}\": {1}")]
    PlayerNotFound(String, String),
    #[error("map not found for alias \"{0}\"")]
    MapNotFound(String),
    #[error("match_start event not found before player_stat")]
    NoActiveMap,
}

pub async fn save_events(
    db: &Dal,
    match_row: &MatchRow,
    log_name: &str,
    events: &[LogEvent],
) -> Result<(), SaveError> {
    if db
        .match_maps
        .log_name_exists_for_match(match_row.id, log_name)
        .await?
    {
        return Err(SaveError::DuplicateLog(log_name.to_string(), match_row.id));
    }

    for event in events {
        if let LogEvent::GenericEvent(generic) = event {
            if generic.event_name == "meta" {
                let version = generic.fields.get(2).map(|s| s.as_str()).unwrap_or("");
                if version != SUPPORTED_LOG_VERSION {
                    return Err(SaveError::UnsupportedLogVersion(
                        version.to_string(),
                        SUPPORTED_LOG_VERSION.to_string(),
                    ));
                }
                break;
            }
        }
    }

    // Build team label → team_id mapping from player_joined events (read-only, outside transaction).
    let mut team_label_to_id: HashMap<String, i64> = HashMap::new();
    for event in events {
        if let LogEvent::GenericEvent(generic) = event {
            if generic.event_name == "player_joined" {
                if let (Some(nickname), Some(label)) =
                    (generic.fields.get(3), generic.fields.get(4))
                {
                    if !team_label_to_id.contains_key(label.as_str()) {
                        if let Some(team_id) = db
                            .players
                            .find_team_id_by_tournament_and_nickname(
                                match_row.tournament_id,
                                nickname,
                            )
                            .await?
                        {
                            team_label_to_id.insert(label.clone(), team_id);
                        }
                    }
                }
            }
        }
    }

    let map_order_offset = db.match_maps.max_map_order_for_match(match_row.id).await?;

    let mut tx = db.db_pool.begin().await?;
    let mut ctx = SaveContext {
        team_label_to_id,
        current_map_order: map_order_offset,
        ..SaveContext::new()
    };

    for event in events {
        match event {
            LogEvent::GenericEvent(generic) if generic.event_name == "match_start" => {
                // fields: [timestamp, match_start, elapsed, map_name, mode, team1, team2]
                let map_name = generic.fields.get(3).map(|s| s.as_str()).unwrap_or("");

                let game_map_id = db
                    .match_maps
                    .find_game_map_id_by_alias_with_executor(&mut *tx, map_name)
                    .await?
                    .ok_or_else(|| SaveError::MapNotFound(map_name.to_string()))?;

                ctx.current_map_order += 1;
                ctx.current_round = 0;
                ctx.current_team1_label = generic.fields.get(5).cloned();
                ctx.current_team2_label = generic.fields.get(6).cloned();

                let match_map_id = db
                    .match_maps
                    .insert(
                        &mut *tx,
                        match_row.id,
                        game_map_id,
                        ctx.current_map_order,
                        0,
                        0,
                        log_name,
                    )
                    .await?;

                ctx.current_match_map_id = Some(match_map_id);
            }

            LogEvent::GenericEvent(generic) if generic.event_name == "match_end" => {
                // fields: [timestamp, match_end, elapsed, rounds, score_team1, score_team2]
                if let (Some(match_map_id), Some(score1_str), Some(score2_str)) = (
                    ctx.current_match_map_id,
                    generic.fields.get(4),
                    generic.fields.get(5),
                ) {
                    let score1: i64 = score1_str.parse().unwrap_or(0);
                    let score2: i64 = score2_str.parse().unwrap_or(0);

                    let (home_score, away_score) =
                        if let Some(team1_label) = &ctx.current_team1_label {
                            match ctx.team_label_to_id.get(team1_label.as_str()) {
                                Some(&id) if id == match_row.home_team_id => (score1, score2),
                                _ => (score2, score1),
                            }
                        } else {
                            (score1, score2)
                        };

                    db.match_maps
                        .update_scores_with_executor(&mut *tx, match_map_id, home_score, away_score)
                        .await?;
                }

                if let Some(match_map_id) = ctx.current_match_map_id {
                    save_single_event(db, &mut *tx, match_map_id, event).await?;
                }
            }

            LogEvent::RoundStartEvent(round_start) => {
                if let Some(round) = round_start.round {
                    ctx.current_round = round;
                }
                if let Some(match_map_id) = ctx.current_match_map_id {
                    save_single_event(db, &mut *tx, match_map_id, event).await?;
                }
            }

            LogEvent::PlayerStatEvent(player_stat) => {
                let match_map_id = ctx.current_match_map_id.ok_or(SaveError::NoActiveMap)?;
                let round = player_stat.round.unwrap_or(ctx.current_round);
                let team = fetch_team(
                    db,
                    &mut *tx,
                    &ctx,
                    &match_row.tournament_id,
                    &player_stat.team,
                )
                .await?;
                let player_id = fetch_player_id(db, &mut *tx, &team, &player_stat.player).await?;
                let hero_id = fetch_hero_id(db, &mut *tx, &player_stat.hero).await?;

                db.match_player_statistics
                    .insert_with_executor(
                        &mut *tx,
                        match_map_id,
                        round,
                        player_id,
                        hero_id,
                        &player_stat.stats,
                    )
                    .await
                    .map_err(|e| {
                        tracing::error!(
                            match_map_id, round, player_id, hero_id,
                            stats_len = player_stat.stats.len(),
                            error = ?e,
                            "failed inserting match_player_statistics"
                        );
                        e
                    })?;
            }

            _ => {
                if let Some(match_map_id) = ctx.current_match_map_id {
                    save_single_event(db, &mut *tx, match_map_id, event).await?;
                }
            }
        }
    }

    tx.commit().await?;
    Ok(())
}

async fn fetch_team(
    db: &Dal,
    tx: &mut SqliteConnection,
    ctx: &SaveContext,
    tournament_id: &i64,
    team_label: &str,
) -> Result<TeamRow, SaveError> {
    // First, try resolving via the workshop label → team_id mapping built from player_joined events.
    if let Some(&team_id) = ctx.team_label_to_id.get(team_label) {
        return db
            .teams
            .get_by_id_with_executor(&mut *tx, team_id)
            .await?
            .ok_or_else(|| {
                SaveError::TeamNotFound(team_label.to_string(), tournament_id.to_string())
            });
    }

    // Fallback: try matching by the DB team name directly (for logs where labels equal team names).
    db.teams
        .get_by_tournament_and_name_with_executor(&mut *tx, tournament_id, team_label)
        .await?
        .ok_or_else(|| SaveError::TeamNotFound(team_label.to_string(), tournament_id.to_string()))
}

async fn fetch_player_id(
    db: &Dal,
    tx: &mut SqliteConnection,
    team: &TeamRow,
    player: &str,
) -> Result<i64, SaveError> {
    db.players
        .find_id_by_team_and_nickname_with_executor(&mut *tx, team.id, player)
        .await?
        .ok_or_else(|| SaveError::PlayerNotFound(team.name.clone(), player.to_string()))
}

async fn fetch_hero_id(db: &Dal, tx: &mut SqliteConnection, hero: &str) -> Result<i64, SaveError> {
    db.heroes
        .find_id_by_any_localized_name_with_executor(&mut *tx, hero)
        .await?
        .ok_or_else(|| SaveError::HeroNotFound(hero.to_string()))
}

async fn save_single_event(
    db: &Dal,
    tx: &mut SqliteConnection,
    match_map_id: i64,
    event: &LogEvent,
) -> Result<(), SaveError> {
    if matches!(event, LogEvent::PlayerStatEvent(_)) {
        return Ok(());
    }

    let time = *event.timestamp();
    let event_name = event.event_name();
    let data = serde_json::to_value(event)?;

    db.match_events
        .insert_with_executor(&mut *tx, match_map_id, time, event_name, data)
        .await?;

    Ok(())
}
