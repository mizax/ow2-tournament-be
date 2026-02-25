use actix_web::{HttpResponse, Responder, get, web};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::api::error::ApiError;
use crate::dal::Dal;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/matches")
            .service(get_match)
            .service(get_match_stats),
    );
}

#[derive(Serialize)]
struct MapScore {
    map_order: i64,
    map_name: Option<String>,
    mode_name: Option<String>,
    home_score: i64,
    away_score: i64,
}

#[derive(Serialize)]
struct MatchDetail {
    id: i64,
    home_team: String,
    away_team: String,
    home_score: Option<i64>,
    away_score: Option<i64>,
    maps: Vec<MapScore>,
}

#[derive(sqlx::FromRow)]
struct MatchMapFlatRow {
    id: i64,
    home_score: Option<i64>,
    away_score: Option<i64>,
    home_team: String,
    away_team: String,
    map_id: Option<i64>,
    map_order: Option<i64>,
    map_home: Option<i64>,
    map_away: Option<i64>,
    map_name: Option<String>,
    mode_name: Option<String>,
}

#[get("/{match_id}")]
async fn get_match(
    db: web::Data<Arc<Dal>>,
    match_id: web::Path<i64>,
) -> actix_web::Result<impl Responder, ApiError> {
    let match_id = match_id.into_inner();

    let rows = sqlx::query_as::<_, MatchMapFlatRow>(
        r#"
        SELECT m.id, m.home_score, m.away_score,
               ht.name as home_team, at.name as away_team,
               mm.id as map_id, mm.map_order, mm.home_score as map_home, mm.away_score as map_away,
               maps.name as map_name, modes.name as mode_name
        FROM matches m
        JOIN teams ht ON m.home_team_id = ht.id
        JOIN teams at ON m.away_team_id = at.id
        LEFT JOIN match_maps mm ON mm.match_id = m.id
        LEFT JOIN game_maps gm ON mm.game_map_id = gm.id
        LEFT JOIN maps ON gm.map_id = maps.id
        LEFT JOIN modes ON gm.mode_id = modes.id
        WHERE m.id = ?1 AND m.deleted_at IS NULL
        ORDER BY mm.map_order
        "#,
    )
    .bind(match_id)
    .fetch_all(&db.db_pool)
    .await
    .map_err(|e| {
        log::error!("Failed to load match {}: {}", match_id, e);
        ApiError::InternalError {
            error: e.to_string(),
        }
    })?;

    if rows.is_empty() {
        return Err(ApiError::NotFound);
    }

    let first = &rows[0];
    let maps = rows
        .iter()
        .filter_map(|row| {
            row.map_order.map(|map_order| MapScore {
                map_order,
                map_name: row.map_name.clone(),
                mode_name: row.mode_name.clone(),
                home_score: row.map_home.unwrap_or(0),
                away_score: row.map_away.unwrap_or(0),
            })
        })
        .collect();

    let detail = MatchDetail {
        id: first.id,
        home_team: first.home_team.clone(),
        away_team: first.away_team.clone(),
        home_score: first.home_score,
        away_score: first.away_score,
        maps,
    };

    Ok(HttpResponse::Ok().json(detail))
}

#[derive(Serialize)]
struct PlayerStats {
    player_id: i64,
    nickname: String,
    team_id: i64,
    team_name: String,
    hero_name: String,
    elims: Option<i64>,
    kills: Option<i64>,
    deaths: Option<i64>,
    damage: Option<f64>,
    hero_damage: Option<f64>,
    healing: Option<f64>,
    damage_taken: Option<f64>,
    damage_blocked: Option<f64>,
    ults_earned: Option<i64>,
    ults_used: Option<i64>,
    time_played: Option<f64>,
}

#[derive(Serialize)]
struct RoundStats {
    round: Option<i64>,
    players: Vec<PlayerStats>,
}

#[derive(Serialize)]
struct MapStats {
    map_order: i64,
    map_name: Option<String>,
    mode_name: Option<String>,
    rounds: Vec<RoundStats>,
}

#[derive(Serialize)]
struct MatchStatsResponse {
    match_id: i64,
    home_team: String,
    home_team_id: i64,
    away_team: String,
    away_team_id: i64,
    maps: Vec<MapStats>,
}

#[derive(sqlx::FromRow)]
struct MatchTeamInfoRow {
    home_team: String,
    home_team_id: i64,
    away_team: String,
    away_team_id: i64,
}

#[derive(sqlx::FromRow)]
struct PlayerStatFlatRow {
    map_order: i64,
    round: Option<i64>,
    team_name: String,
    team_id: i64,
    player_id: i64,
    nickname: String,
    hero_name: String,
    eliminations: Option<i64>,
    final_blows: Option<i64>,
    deaths: Option<i64>,
    all_damage: Option<f64>,
    hero_damage: Option<f64>,
    healing_dealt: Option<f64>,
    damage_taken: Option<f64>,
    damage_blocked: Option<f64>,
    ultimates_earned: Option<i64>,
    ultimates_used: Option<i64>,
    hero_time_played: Option<f64>,
    map_name: Option<String>,
    mode_name: Option<String>,
}

#[get("/{match_id}/stats")]
async fn get_match_stats(
    db: web::Data<Arc<Dal>>,
    match_id: web::Path<i64>,
) -> actix_web::Result<impl Responder, ApiError> {
    let match_id = match_id.into_inner();

    let match_info = sqlx::query_as::<_, MatchTeamInfoRow>(
        r#"
        SELECT ht.name as home_team, ht.id as home_team_id,
               at.name as away_team, at.id as away_team_id
        FROM matches m
        JOIN teams ht ON m.home_team_id = ht.id
        JOIN teams at ON m.away_team_id = at.id
        WHERE m.id = ?1 AND m.deleted_at IS NULL
        "#,
    )
    .bind(match_id)
    .fetch_optional(&db.db_pool)
    .await
    .map_err(|e| {
        log::error!("Failed to load match info {}: {}", match_id, e);
        ApiError::InternalError {
            error: e.to_string(),
        }
    })?
    .ok_or(ApiError::NotFound)?;

    let rows = sqlx::query_as::<_, PlayerStatFlatRow>(
        r#"
        SELECT mm.map_order, mps.round,
               t.name as team_name, t.id as team_id,
               p.id as player_id, p.nickname,
               h.name as hero_name,
               mps.eliminations, mps.final_blows, mps.deaths,
               mps.all_damage, mps.hero_damage, mps.healing_dealt,
               mps.damage_taken, mps.damage_blocked,
               mps.ultimates_earned, mps.ultimates_used,
               mps.hero_time_played,
               maps.name as map_name, modes.name as mode_name
        FROM match_player_statistics mps
        JOIN match_maps mm ON mps.match_map_id = mm.id
        JOIN players p ON mps.player_id = p.id
        JOIN teams t ON p.team_id = t.id
        JOIN heroes h ON mps.hero_id = h.id
        LEFT JOIN game_maps gm ON mm.game_map_id = gm.id
        LEFT JOIN maps ON gm.map_id = maps.id
        LEFT JOIN modes ON gm.mode_id = modes.id
        WHERE mm.match_id = ?1
        ORDER BY mm.map_order, mps.round, t.name, p.nickname
        "#,
    )
    .bind(match_id)
    .fetch_all(&db.db_pool)
    .await
    .map_err(|e| {
        log::error!("Failed to load match stats {}: {}", match_id, e);
        ApiError::InternalError {
            error: e.to_string(),
        }
    })?;

    // Group: map_order -> (map_name, mode_name, round -> players)
    let mut maps_map: BTreeMap<
        i64,
        (
            Option<String>,
            Option<String>,
            BTreeMap<Option<i64>, Vec<PlayerStats>>,
        ),
    > = BTreeMap::new();

    for row in rows {
        let map_entry = maps_map
            .entry(row.map_order)
            .or_insert_with(|| (row.map_name.clone(), row.mode_name.clone(), BTreeMap::new()));
        let round_entry = map_entry.2.entry(row.round).or_insert_with(Vec::new);
        round_entry.push(PlayerStats {
            player_id: row.player_id,
            nickname: row.nickname,
            team_id: row.team_id,
            team_name: row.team_name,
            hero_name: row.hero_name,
            elims: row.eliminations,
            kills: row.final_blows,
            deaths: row.deaths,
            damage: row.all_damage,
            hero_damage: row.hero_damage,
            healing: row.healing_dealt,
            damage_taken: row.damage_taken,
            damage_blocked: row.damage_blocked,
            ults_earned: row.ultimates_earned,
            ults_used: row.ultimates_used,
            time_played: row.hero_time_played,
        });
    }

    let maps = maps_map
        .into_iter()
        .map(|(map_order, (map_name, mode_name, rounds_map))| {
            let rounds = rounds_map
                .into_iter()
                .map(|(round, players)| RoundStats { round, players })
                .collect();
            MapStats {
                map_order,
                map_name,
                mode_name,
                rounds,
            }
        })
        .collect();

    let response = MatchStatsResponse {
        match_id,
        home_team: match_info.home_team,
        home_team_id: match_info.home_team_id,
        away_team: match_info.away_team,
        away_team_id: match_info.away_team_id,
        maps,
    };

    Ok(HttpResponse::Ok().json(response))
}
