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
    /// Last seen cumulative player_stat per (player_id, hero_id) for the current map.
    /// Blizzard emits cumulative stats per round; we subtract the previous snapshot to get per-round deltas.
    pub last_player_stats: HashMap<(i64, i64), Vec<String>>,
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
            last_player_stats: HashMap::new(),
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
                ctx.last_player_stats.clear();

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

                let prev = ctx
                    .last_player_stats
                    .get(&(player_id, hero_id))
                    .map(|v| v.as_slice())
                    .unwrap_or(&[]);
                let delta_stats = compute_delta_stats(&player_stat.stats, prev);
                ctx.last_player_stats
                    .insert((player_id, hero_id), player_stat.stats.clone());

                db.match_player_statistics
                    .insert_with_executor(
                        &mut *tx,
                        match_map_id,
                        round,
                        player_id,
                        hero_id,
                        &delta_stats,
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

/// Stat indices that are cumulative sums — we store per-round deltas for these.
/// Index 15 (multikill_best) is a running MAX, kept as-is.
/// Indices 22, 23, 24, 31 (accuracy %) are recomputed from delta shot counters below.
const CUMULATIVE_SUM_INDICES: &[usize] = &[
    0,  // eliminations
    1,  // final_blows
    2,  // deaths
    3,  // all_damage
    4,  // barrier_damage
    5,  // hero_damage
    6,  // healing_dealt
    7,  // healing_received
    8,  // self_healing
    9,  // damage_taken
    10, // damage_blocked
    11, // defensive_assists
    12, // offensive_assists
    13, // ultimates_earned
    14, // ultimates_used
    // 15: multikill_best — kept as cumulative MAX
    16, // multikills
    17, // solo_kills
    18, // objective_kills
    19, // environmental_kills
    20, // environmental_deaths
    21, // critical_hits
    // 22: critical_hit_accuracy — recomputed
    // 23: scoped_accuracy — recomputed
    // 24: scoped_critical_hit_accuracy — recomputed
    25, // scoped_critical_hit_kills
    26, // shots_fired
    27, // shots_hit
    28, // shots_missed
    29, // scoped_shots_fired
    30, // scoped_shots_hit
    // 31: weapon_accuracy — recomputed
    32, // hero_time_played
];

fn stat_as_f64(stats: &[String], i: usize) -> f64 {
    stats
        .get(i)
        .and_then(|s| s.trim().parse::<f64>().ok())
        .unwrap_or(0.0)
}

fn delta_f64(current: &[String], prev: &[String], i: usize) -> f64 {
    (stat_as_f64(current, i) - stat_as_f64(prev, i)).max(0.0)
}

fn fmt_delta(d: f64) -> String {
    if d.fract() == 0.0 {
        format!("{}", d as i64)
    } else {
        format!("{}", d)
    }
}

fn pct_of(num: f64, den: f64) -> String {
    if den > 0.0 {
        format!("{:.2}", num / den * 100.0)
    } else {
        "0".to_string()
    }
}

/// Convert Blizzard's cumulative player_stat snapshot into a per-round delta.
/// `prev` is the previous snapshot for the same (player, hero) on this map,
/// or an empty slice for the first round (delta == raw value).
/// All deltas are clamped to 0 to handle Blizzard data inconsistencies.
fn compute_delta_stats(current: &[String], prev: &[String]) -> Vec<String> {
    let len = current.len();
    let mut result = current.to_vec();

    for &i in CUMULATIVE_SUM_INDICES {
        if i < len {
            result[i] = fmt_delta(delta_f64(current, prev, i));
        }
    }

    // Recompute accuracy fields from delta shot counters.
    let d_shots_fired = delta_f64(current, prev, 26);
    let d_shots_hit = delta_f64(current, prev, 27);
    let d_scoped_shots_fired = delta_f64(current, prev, 29);
    let d_scoped_shots_hit = delta_f64(current, prev, 30);
    let d_crit_hits = delta_f64(current, prev, 21);
    let d_scoped_crit_kills = delta_f64(current, prev, 25);

    if 22 < len {
        result[22] = pct_of(d_crit_hits, d_shots_fired);
    } // critical_hit_accuracy
    if 23 < len {
        result[23] = pct_of(d_scoped_shots_hit, d_scoped_shots_fired);
    } // scoped_accuracy
    if 24 < len {
        result[24] = pct_of(d_scoped_crit_kills, d_scoped_shots_hit);
    } // scoped_critical_hit_accuracy
    if 31 < len {
        result[31] = pct_of(d_shots_hit, d_shots_fired);
    } // weapon_accuracy

    result
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
