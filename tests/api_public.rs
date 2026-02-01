use std::sync::Arc;

use actix_web::{App, web};
use actix_web::test;
use serde_json::Value;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;

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
