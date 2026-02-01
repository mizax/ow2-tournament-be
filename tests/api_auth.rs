use std::sync::Arc;

use actix_web::{App, web};
use actix_web::test;
use httpmock::Method::POST;
use httpmock::Method::GET;
use httpmock::MockServer;
use serde_json::Value;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;

use ow2_tournament_be::api;
use ow2_tournament_be::api::auth::state_store::OAuthStateStore;
use ow2_tournament_be::config::{App as AppConfig, BattleNetConfig, Config, CustomSqliteConnectOptions};
use ow2_tournament_be::dal::dal::Dal;
use ow2_tournament_be::storage::StorageConfig;

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

fn build_config() -> Config {
    Config {
        server_addr: "127.0.0.1:0".to_string(),
        actix_workers: 1,
        sqlite: CustomSqliteConnectOptions {
            filename: ":memory:".to_string(),
            text_extension: "".to_string(),
        },
        app: AppConfig {
            main_host: "https://main.example".to_string(),
            alt_host: "https://alt.example".to_string(),
        },
        storage: StorageConfig::FileSystem {
            root_dir: "./storage".to_string(),
            shard_levels: None,
            shard_chars: None,
            serve_url: None,
        },
        battlenet: BattleNetConfig {
            client_id: "client".to_string(),
            client_secret: "secret".to_string(),
            redirect_uri: "/api/public/v1/auth/battlenet/callback".to_string(),
            jwks_url: "https://example.com/jwks".to_string(),
            token_url: None,
            userinfo_url: None,
        },
    }
}

#[actix_web::test]
async fn auth_battlenet_returns_auth_url() {
    let db = TestDb::new().await;
    let dal = Dal::from_pool(db.pool.clone());
    let config = build_config();
    let state_store = OAuthStateStore::new(std::time::Duration::from_secs(3600));

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(Arc::new(config)))
            .app_data(web::Data::new(state_store))
            .app_data(web::Data::new(Arc::new(dal)))
            .configure(api::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/public/v1/auth/battlenet")
        .insert_header(("host", "main.example"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    let auth_url = body["auth_url"].as_str().expect("auth_url");
    assert!(auth_url.starts_with("https://oauth.battle.net/authorize?"));
    assert!(auth_url.contains("redirect_uri=https%3A%2F%2Fmain.example%2Fapi%2Fpublic%2Fv1%2Fauth%2Fbattlenet%2Fcallback"));
}

#[actix_web::test]
async fn auth_battlenet_callback_missing_state_returns_400() {
    let db = TestDb::new().await;
    let dal = Dal::from_pool(db.pool.clone());
    let config = build_config();
    let state_store = OAuthStateStore::new(std::time::Duration::from_secs(3600));

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(Arc::new(config)))
            .app_data(web::Data::new(state_store))
            .app_data(web::Data::new(Arc::new(dal)))
            .configure(api::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/public/v1/auth/battlenet/callback?code=abc")
        .insert_header(("host", "main.example"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
}

#[actix_web::test]
async fn auth_battlenet_callback_error_returns_400() {
    let db = TestDb::new().await;
    let dal = Dal::from_pool(db.pool.clone());
    let config = build_config();
    let state_store = OAuthStateStore::new(std::time::Duration::from_secs(3600));

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(Arc::new(config)))
            .app_data(web::Data::new(state_store))
            .app_data(web::Data::new(Arc::new(dal)))
            .configure(api::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri("/api/public/v1/auth/battlenet/callback?error=access_denied&error_description=denied")
        .insert_header(("host", "main.example"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
}

#[actix_web::test]
async fn auth_battlenet_callback_success_creates_user() {
    let db = TestDb::new().await;
    let dal = Dal::from_pool(db.pool.clone());
    let mut config = build_config();
    let state_store = OAuthStateStore::new(std::time::Duration::from_secs(3600));
    let state = "state-ok".to_string();
    state_store.add_state(state.clone());

    let server = MockServer::start_async().await;
    config.battlenet.token_url = Some(format!("{}/token", server.base_url()));
    config.battlenet.userinfo_url = Some(format!("{}/userinfo", server.base_url()));

    server
        .mock_async(|when, then| {
            when.method(POST).path("/token");
            then.status(200).json_body(serde_json::json!({
                "access_token": "access",
                "token_type": "bearer",
                "expires_in": 3600,
                "scope": "openid"
            }));
        })
        .await;

    server
        .mock_async(|when, then| {
            when.method(GET).path("/userinfo");
            then.status(200).json_body(serde_json::json!({
                "id": 1234,
                "battletag": "User#9999"
            }));
        })
        .await;

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(Arc::new(config)))
            .app_data(web::Data::new(state_store))
            .app_data(web::Data::new(Arc::new(dal)))
            .configure(api::configure),
    )
    .await;

    let req = test::TestRequest::get()
        .uri(&format!("/api/public/v1/auth/battlenet/callback?code=abc&state={}", state))
        .insert_header(("host", "main.example"))
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["user"]["id"], 1234);
    assert_eq!(body["user"]["battletag"], "User#9999");
    let roles = body["user"]["roles"].as_array().expect("roles array");
    assert!(roles.iter().any(|r| r == "normal_user"));
}
