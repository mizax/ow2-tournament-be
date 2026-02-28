use std::sync::Arc;

use actix_web::test;
use actix_web::{App, web};
use serde_json::Value;
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

use ow2_tournament_be::api;
use ow2_tournament_be::dal::dal::Dal;

struct TestDb {
    _dir: tempfile::TempDir,
    pool: SqlitePool,
}

impl TestDb {
    async fn new() -> Self {
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

async fn insert_tournament(
    pool: &SqlitePool,
    title: &str,
    sef_title: &str,
    discipline: &str,
    format: &str,
    dates_json: &str,
    prize_amount: f64,
    prize_currency: &str,
) -> i64 {
    sqlx::query(
        r#"
        INSERT INTO tournaments (
            title, sef_title, discipline, format, dates_json, prize_pool_total_amount, prize_pool_currency
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
    )
    .bind(title)
    .bind(sef_title)
    .bind(discipline)
    .bind(format)
    .bind(dates_json)
    .bind(prize_amount)
    .bind(prize_currency)
    .execute(pool)
    .await
    .expect("insert tournament");

    let tournament_id: i64 = sqlx::query_scalar("SELECT last_insert_rowid()")
        .fetch_one(pool)
        .await
        .expect("last_insert_rowid");

    let config_json = serde_json::json!({
        "type": "online",
        "schedule": [],
        "prize_pool": {
            "currency": null,
            "places": null
        }
    })
    .to_string();

    sqlx::query(
        r#"
        INSERT INTO tournament_configuration (tournament_id, configuration_json)
        VALUES (?1, ?2)
        "#,
    )
    .bind(tournament_id)
    .bind(config_json)
    .execute(pool)
    .await
    .expect("insert tournament_configuration");

    tournament_id
}

async fn set_tournament_placements(
    pool: &SqlitePool,
    tournament_id: i64,
    placements: serde_json::Value,
) {
    let config_json = serde_json::json!({
        "type": "online",
        "schedule": [],
        "prize_pool": {
            "currency": null,
            "places": null
        },
        "results": {
            "placements": placements
        }
    })
    .to_string();

    sqlx::query(
        r#"
        UPDATE tournament_configuration
        SET configuration_json = ?1
        WHERE tournament_id = ?2
        "#,
    )
    .bind(config_json)
    .bind(tournament_id)
    .execute(pool)
    .await
    .expect("update tournament_configuration");
}

async fn insert_team(pool: &SqlitePool, tournament_id: i64, name: &str) -> i64 {
    sqlx::query(
        r#"
        INSERT INTO teams (tournament_id, name)
        VALUES (?1, ?2)
        "#,
    )
    .bind(tournament_id)
    .bind(name)
    .execute(pool)
    .await
    .expect("insert team");

    sqlx::query_scalar("SELECT last_insert_rowid()")
        .fetch_one(pool)
        .await
        .expect("last_insert_rowid")
}

async fn insert_player(pool: &SqlitePool, team_id: i64, nickname: &str, role: Option<&str>) -> i64 {
    sqlx::query(
        r#"
        INSERT INTO players (team_id, nickname, role)
        VALUES (?1, ?2, ?3)
        "#,
    )
    .bind(team_id)
    .bind(nickname)
    .bind(role)
    .execute(pool)
    .await
    .expect("insert player");

    sqlx::query_scalar("SELECT last_insert_rowid()")
        .fetch_one(pool)
        .await
        .expect("last_insert_rowid")
}

async fn insert_match(
    pool: &SqlitePool,
    tournament_id: i64,
    home_team_id: i64,
    away_team_id: i64,
    home_score: i64,
    away_score: i64,
) -> i64 {
    sqlx::query(
        r#"
        INSERT INTO matches (tournament_id, home_team_id, away_team_id, home_score, away_score)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
    )
    .bind(tournament_id)
    .bind(home_team_id)
    .bind(away_team_id)
    .bind(home_score)
    .bind(away_score)
    .execute(pool)
    .await
    .expect("insert match");

    sqlx::query_scalar("SELECT last_insert_rowid()")
        .fetch_one(pool)
        .await
        .expect("last_insert_rowid")
}

async fn first_game_map_id(pool: &SqlitePool) -> i64 {
    sqlx::query_scalar("SELECT id FROM game_maps ORDER BY id LIMIT 1")
        .fetch_one(pool)
        .await
        .expect("game_maps has seed rows")
}

async fn insert_match_map(
    pool: &SqlitePool,
    match_id: i64,
    game_map_id: i64,
    map_order: i64,
    home_score: i64,
    away_score: i64,
) -> i64 {
    sqlx::query(
        r#"
        INSERT INTO match_maps (match_id, game_map_id, map_order, home_score, away_score)
        VALUES (?1, ?2, ?3, ?4, ?5)
        "#,
    )
    .bind(match_id)
    .bind(game_map_id)
    .bind(map_order)
    .bind(home_score)
    .bind(away_score)
    .execute(pool)
    .await
    .expect("insert match_map");

    sqlx::query_scalar("SELECT last_insert_rowid()")
        .fetch_one(pool)
        .await
        .expect("last_insert_rowid")
}

async fn insert_match_player_stats(
    pool: &SqlitePool,
    match_map_id: i64,
    player_id: i64,
    eliminations: i64,
    deaths: i64,
    all_damage: f64,
    healing_dealt: f64,
    hero_time_played: f64,
) {
    sqlx::query(
        r#"
        INSERT INTO match_player_statistics (
            match_map_id,
            round,
            player_id,
            hero_id,
            eliminations,
            deaths,
            all_damage,
            healing_dealt,
            hero_time_played
        ) VALUES (?1, ?2, ?3, NULL, ?4, ?5, ?6, ?7, ?8)
        "#,
    )
    .bind(match_map_id)
    .bind(1_i64)
    .bind(player_id)
    .bind(eliminations)
    .bind(deaths)
    .bind(all_damage)
    .bind(healing_dealt)
    .bind(hero_time_played)
    .execute(pool)
    .await
    .expect("insert match_player_statistics");
}

#[actix_web::test]
async fn get_tournaments_returns_list() {
    let db = TestDb::new().await;
    insert_tournament(
        &db.pool,
        "Public Cup",
        "public-cup",
        "ow2",
        "5v5",
        r#"["2026-02-01"]"#,
        50.0,
        "USD",
    )
    .await;

    let dal = Dal::from_pool(db.pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(Arc::new(dal)))
            .configure(api::configure),
    )
    .await;
    let req = test::TestRequest::get()
        .uri("/api/public/v1/tournaments")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    let items = body.as_array().expect("array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["uri"], "public-cup");
    assert_eq!(items[0]["prize_pool"], "50 USD");
}

#[actix_web::test]
async fn get_tournaments_returns_podium_for_finished_data() {
    let db = TestDb::new().await;
    let tournament_id = insert_tournament(
        &db.pool,
        "Winners Cup",
        "winners-cup",
        "ow2",
        "5v5",
        r#"["2026-02-01"]"#,
        50.0,
        "USD",
    )
    .await;

    set_tournament_placements(
        &db.pool,
        tournament_id,
        serde_json::json!([
            { "place": 2, "team_name": "Team Beta" },
            { "place": 1, "team_name": "Team Alpha" },
            { "place": 4, "team_name": "Team Delta" },
            { "place": 3, "team_name": "Team Gamma" }
        ]),
    )
    .await;

    let dal = Dal::from_pool(db.pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(Arc::new(dal)))
            .configure(api::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/public/v1/tournaments")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    let items = body.as_array().expect("array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["uri"], "winners-cup");

    let podium = items[0]["podium"].as_array().expect("podium array");
    assert_eq!(podium.len(), 3);
    assert_eq!(podium[0]["place"], 1);
    assert_eq!(podium[0]["team_name"], "Team Alpha");
    assert_eq!(podium[1]["place"], 2);
    assert_eq!(podium[1]["team_name"], "Team Beta");
    assert_eq!(podium[2]["place"], 3);
    assert_eq!(podium[2]["team_name"], "Team Gamma");
}

#[actix_web::test]
async fn get_tournament_returns_details() {
    let db = TestDb::new().await;
    insert_tournament(
        &db.pool,
        "Details Cup",
        "details-cup",
        "ow2",
        "5v5",
        r#"["2026-02-02"]"#,
        0.0,
        "",
    )
    .await;

    let dal = Dal::from_pool(db.pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(Arc::new(dal)))
            .configure(api::configure),
    )
    .await;
    let req = test::TestRequest::get()
        .uri("/api/public/v1/tournaments/details-cup")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["id"], "details-cup");
    assert_eq!(body["title"], "Details Cup");
    assert_eq!(body["discipline"], "ow2");
}

#[actix_web::test]
async fn get_tournament_returns_404_for_missing() {
    let db = TestDb::new().await;
    let dal = Dal::from_pool(db.pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(Arc::new(dal)))
            .configure(api::configure),
    )
    .await;
    let req = test::TestRequest::get()
        .uri("/api/public/v1/tournaments/missing")
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 404);
}

#[actix_web::test]
async fn get_player_returns_public_profile() {
    let db = TestDb::new().await;
    let tournament_id = insert_tournament(
        &db.pool,
        "Players Cup",
        "players-cup",
        "ow2",
        "5v5",
        r#"["2026-03-01"]"#,
        10.0,
        "USD",
    )
    .await;
    let team_id = insert_team(&db.pool, tournament_id, "Alpha").await;
    let player_id = insert_player(&db.pool, team_id, "BestPlayer", Some("damage")).await;

    let dal = Dal::from_pool(db.pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(Arc::new(dal)))
            .configure(api::configure),
    )
    .await;
    let req = test::TestRequest::get()
        .uri(&format!("/api/public/v1/players/{}", player_id))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["id"], player_id);
    assert_eq!(body["nickname"], "BestPlayer");
    assert_eq!(body["role"], "damage");
    assert_eq!(body["team_name"], "Alpha");
    assert_eq!(body["tournament_sef"], "players-cup");
}

#[actix_web::test]
async fn get_player_matches_returns_aggregated_stats() {
    let db = TestDb::new().await;
    let tournament_id = insert_tournament(
        &db.pool,
        "Stats Cup",
        "stats-cup",
        "ow2",
        "5v5",
        r#"["2026-03-02"]"#,
        25.0,
        "USD",
    )
    .await;
    let home_team_id = insert_team(&db.pool, tournament_id, "Home").await;
    let away_team_id = insert_team(&db.pool, tournament_id, "Away").await;
    let player_id = insert_player(&db.pool, home_team_id, "Carry", Some("damage")).await;
    let match_id = insert_match(&db.pool, tournament_id, home_team_id, away_team_id, 3, 2).await;
    let game_map_id = first_game_map_id(&db.pool).await;
    let map1_id = insert_match_map(&db.pool, match_id, game_map_id, 1, 2, 1).await;
    let map2_id = insert_match_map(&db.pool, match_id, game_map_id, 2, 1, 1).await;
    insert_match_player_stats(&db.pool, map1_id, player_id, 10, 4, 5000.0, 1200.0, 600.0).await;
    insert_match_player_stats(&db.pool, map2_id, player_id, 5, 3, 3500.0, 800.0, 500.0).await;

    let dal = Dal::from_pool(db.pool.clone());
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(Arc::new(dal)))
            .configure(api::configure),
    )
    .await;
    let req = test::TestRequest::get()
        .uri(&format!("/api/public/v1/players/{}/matches", player_id))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    let items = body.as_array().expect("array");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0]["match_id"], match_id);
    assert_eq!(items[0]["maps_played"], 2);
    assert_eq!(items[0]["kills"], 15);
    assert_eq!(items[0]["deaths"], 7);
    assert_eq!(items[0]["damage"], 8500.0);
    assert_eq!(items[0]["healing"], 2000.0);
    assert_eq!(items[0]["time_played"], 1100.0);
}
