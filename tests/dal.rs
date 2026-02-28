use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

use ow2_tournament_be::dal::{
    ActionStatus, HeroesRepo, MatchesRepo, NewRegistration, PlayersRepo, RegistrationStatus,
    RegistrationsRepo, RoleValue, TeamsRepo, TournamentManagersRepo, TournamentsRepo,
    UpdateTournamentData, UsersRepo,
};

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

async fn insert_user_with_battletag(pool: &SqlitePool, user_id: i64, battletag: &str) -> i64 {
    sqlx::query("INSERT INTO users (id) VALUES (?1)")
        .bind(user_id)
        .execute(pool)
        .await
        .expect("insert user");

    sqlx::query(r#"INSERT INTO user_battletags (user_id, battletag) VALUES (?1, ?2)"#)
        .bind(user_id)
        .bind(battletag)
        .execute(pool)
        .await
        .expect("insert battletag");

    sqlx::query_scalar("SELECT last_insert_rowid()")
        .fetch_one(pool)
        .await
        .expect("last_insert_rowid")
}

async fn insert_tournament_without_config(
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
    .expect("insert tournament without config");

    sqlx::query_scalar("SELECT last_insert_rowid()")
        .fetch_one(pool)
        .await
        .expect("last_insert_rowid")
}

async fn insert_registration(
    pool: &SqlitePool,
    tournament_id: i64,
    user_id: i64,
    user_battletag_id: i64,
    status: &str,
) -> i64 {
    sqlx::query(
        r#"
        INSERT INTO registrations (
            tournament_id,
            user_id,
            user_battletag_id,
            status,
            alt_accounts_json,
            twitch,
            discord,
            primary_role,
            secondary_role,
            guarantors_json,
            additional_info,
            rules_accepted,
            ip_address,
            user_agent
        ) VALUES (
            ?1, ?2, ?3, ?4, '[]', '', '', NULL, NULL, '[]', '', 1, '127.0.0.1', 'tests'
        )
        "#,
    )
    .bind(tournament_id)
    .bind(user_id)
    .bind(user_battletag_id)
    .bind(status)
    .execute(pool)
    .await
    .expect("insert registration");

    sqlx::query_scalar("SELECT last_insert_rowid()")
        .fetch_one(pool)
        .await
        .expect("last_insert_rowid")
}

fn build_registration(
    tournament_id: i64,
    user_id: i64,
    user_battletag_id: i64,
    status: RegistrationStatus,
) -> NewRegistration {
    NewRegistration {
        tournament_id,
        user_id,
        user_battletag_id,
        status,
        alt_accounts_json: "[\"Alt#1111\"]".to_string(),
        twitch: "twitch-user".to_string(),
        discord: "discord-user".to_string(),
        primary_role: Some(RoleValue::Damage),
        secondary_role: Some(RoleValue::Support),
        guarantors_json: "[\"Guarantor#2222\"]".to_string(),
        additional_info: "extra".to_string(),
        rules_accepted: true,
        ip_address: "127.0.0.1".to_string(),
        user_agent: "tests".to_string(),
        decline_reason: None,
    }
}

#[tokio::test]
async fn users_repo_creates_and_reads_user_with_battletags() {
    let db = TestDb::new().await;
    let repo = UsersRepo::new(db.pool.clone());

    let user = repo.create_user(42).await.expect("create_user");
    assert_eq!(user.id, 42);

    let found = repo.find_by_id(42).await.expect("find_by_id");
    assert!(found.is_some());

    repo.insert_battletag(42, "Tester#1234")
        .await
        .expect("insert_battletag");
    let exists = repo
        .battletag_exists(42, "Tester#1234")
        .await
        .expect("battletag_exists");
    assert!(exists);

    repo.upsert_battletag(42, "Tester#1234")
        .await
        .expect("upsert_battletag");

    let battletag_id = repo
        .find_battletag_id(42, "Tester#1234")
        .await
        .expect("find_battletag_id");
    assert!(battletag_id.is_some());

    let battletag = repo
        .find_battletag_by_id(battletag_id.unwrap())
        .await
        .expect("find_battletag_by_id");
    assert!(battletag.is_some());
}

#[tokio::test]
async fn heroes_repo_finds_localized_names() {
    let db = TestDb::new().await;
    let repo = HeroesRepo::new(db.pool.clone());

    let en_id = repo
        .find_id_by_localized_name("Tracer", "en")
        .await
        .expect("find_id_by_localized_name");
    assert!(en_id.is_some());

    let any_id = repo
        .find_id_by_any_localized_name("Tracer")
        .await
        .expect("find_id_by_any_localized_name");
    assert!(any_id.is_some());
}

#[tokio::test]
async fn teams_and_players_repo_lookup() {
    let db = TestDb::new().await;
    let teams_repo = TeamsRepo::new(db.pool.clone());
    let players_repo = PlayersRepo::new(db.pool.clone());

    let tournament_id = insert_tournament(
        &db.pool,
        "Test Cup",
        "test-cup",
        "ow2",
        "5v5",
        r#"["2026-01-01"]"#,
        0.0,
        "",
    )
    .await;

    sqlx::query(r#"INSERT INTO teams (tournament_id, name) VALUES (?1, ?2)"#)
        .bind(tournament_id)
        .bind("Team Alpha")
        .execute(&db.pool)
        .await
        .expect("insert team");

    let team = teams_repo
        .get_by_tournament_and_name(&tournament_id, "Team Alpha")
        .await
        .expect("get_by_tournament_and_name")
        .expect("team not found");

    let player_id = players_repo
        .create(team.id, "PlayerOne")
        .await
        .expect("create player");

    let found_player_id = players_repo
        .find_id_by_team_and_nickname(team.id, "PlayerOne")
        .await
        .expect("find_id_by_team_and_nickname");
    assert_eq!(found_player_id, Some(player_id));
}

#[tokio::test]
async fn matches_repo_reads_match_row() {
    let db = TestDb::new().await;
    let matches_repo = MatchesRepo::new(db.pool.clone());

    let tournament_id = insert_tournament(
        &db.pool,
        "Match Cup",
        "match-cup",
        "ow2",
        "5v5",
        r#"["2026-01-02"]"#,
        0.0,
        "",
    )
    .await;

    sqlx::query("INSERT INTO teams (tournament_id, name) VALUES (?1, ?2)")
        .bind(tournament_id)
        .bind("Home")
        .execute(&db.pool)
        .await
        .expect("insert home team");
    let home_team_id: i64 = sqlx::query_scalar("SELECT last_insert_rowid()")
        .fetch_one(&db.pool)
        .await
        .expect("last_insert_rowid");

    sqlx::query("INSERT INTO teams (tournament_id, name) VALUES (?1, ?2)")
        .bind(tournament_id)
        .bind("Away")
        .execute(&db.pool)
        .await
        .expect("insert away team");
    let away_team_id: i64 = sqlx::query_scalar("SELECT last_insert_rowid()")
        .fetch_one(&db.pool)
        .await
        .expect("last_insert_rowid");

    sqlx::query(
        r#"
        INSERT INTO matches (
            tournament_id, home_team_id, away_team_id, home_score, away_score, duration
        ) VALUES (?1, ?2, ?3, 3, 2, 123)
        "#,
    )
    .bind(tournament_id)
    .bind(home_team_id)
    .bind(away_team_id)
    .execute(&db.pool)
    .await
    .expect("insert match");

    let match_id: i64 = sqlx::query_scalar("SELECT last_insert_rowid()")
        .fetch_one(&db.pool)
        .await
        .expect("last_insert_rowid");

    let exists = matches_repo.exists(match_id).await.expect("exists");
    assert!(exists);

    let row = matches_repo.get_by_id(match_id).await.expect("get_by_id");
    assert!(row.is_some());
    assert_eq!(row.unwrap().id, match_id);
}

#[tokio::test]
async fn tournaments_repo_lists_and_reads() {
    let db = TestDb::new().await;
    let repo = TournamentsRepo::new(db.pool.clone());

    let tournament_id = insert_tournament(
        &db.pool,
        "Prize Cup",
        "prize-cup",
        "ow2",
        "5v5",
        r#"["2026-01-03"]"#,
        100.0,
        "USD",
    )
    .await;

    let user_battletag_id = insert_user_with_battletag(&db.pool, 7, "User#1111").await;
    insert_registration(&db.pool, tournament_id, 7, user_battletag_id, "PENDING").await;
    let user_battletag_id2 = insert_user_with_battletag(&db.pool, 8, "User#2222").await;
    insert_registration(&db.pool, tournament_id, 8, user_battletag_id2, "DECLINED").await;

    let list = repo.list_short().await.expect("list_short");
    assert_eq!(list.len(), 1);
    let item = &list[0];
    assert_eq!(item.uri, "prize-cup");
    assert_eq!(item.dates, vec!["2026-01-03"]);
    assert_eq!(item.prize_pool.as_deref(), Some("100 USD"));
    assert_eq!(item.registration_count, 1);

    let id = repo
        .get_id_by_sef_title("prize-cup")
        .await
        .expect("get_id_by_sef_title");
    assert_eq!(id, Some(tournament_id));

    let details = repo
        .get_by_sef_title("prize-cup")
        .await
        .expect("get_by_sef_title");
    assert!(details.is_some());
    assert_eq!(details.unwrap().id, tournament_id);
}

#[tokio::test]
async fn tournament_managers_repo_queries() {
    let db = TestDb::new().await;
    let repo = TournamentManagersRepo::new(db.pool.clone());

    let tournament_id = insert_tournament(
        &db.pool,
        "Manager Cup",
        "manager-cup",
        "ow2",
        "5v5",
        r#"["2026-01-04"]"#,
        0.0,
        "",
    )
    .await;

    sqlx::query("INSERT INTO users (id) VALUES (?1)")
        .bind(99)
        .execute(&db.pool)
        .await
        .expect("insert user");

    sqlx::query(r#"INSERT INTO tournament_managers (tournament_id, user_id) VALUES (?1, ?2)"#)
        .bind(tournament_id)
        .bind(99)
        .execute(&db.pool)
        .await
        .expect("insert manager");

    let user_battletag_id = insert_user_with_battletag(&db.pool, 100, "User#2222").await;
    let registration_id =
        insert_registration(&db.pool, tournament_id, 100, user_battletag_id, "PENDING").await;

    let is_manager = repo.user_is_manager(99).await.expect("user_is_manager");
    assert!(is_manager);

    let is_for_tournament = repo
        .user_is_manager_for_tournament(tournament_id, 99)
        .await
        .expect("user_is_manager_for_tournament");
    assert!(is_for_tournament);

    let is_for_registration = repo
        .user_is_manager_for_registration(registration_id, 99)
        .await
        .expect("user_is_manager_for_registration");
    assert!(is_for_registration);

    let managed = repo
        .list_managed_tournaments(99)
        .await
        .expect("list_managed_tournaments");
    assert_eq!(managed.len(), 1);
    assert_eq!(managed[0].sef_title, "manager-cup");
    assert_eq!(managed[0].registration_count, 1);
}

#[tokio::test]
async fn registrations_repo_create_and_find() {
    let db = TestDb::new().await;
    let repo = RegistrationsRepo::new(db.pool.clone());

    let tournament_id = insert_tournament(
        &db.pool,
        "Reg Cup",
        "reg-cup",
        "ow2",
        "5v5",
        r#"["2026-01-05"]"#,
        0.0,
        "",
    )
    .await;

    let user_battletag_id = insert_user_with_battletag(&db.pool, 11, "User#3333").await;

    let registration_id = repo
        .create(build_registration(
            tournament_id,
            11,
            user_battletag_id,
            RegistrationStatus::Pending,
        ))
        .await
        .expect("create registration");

    let found = repo
        .find_user_registration(11, tournament_id)
        .await
        .expect("find_user_registration");
    assert!(found.is_some());
    assert_eq!(found.unwrap().id, registration_id);

    let details = repo
        .find_by_id_for_user(registration_id, 11)
        .await
        .expect("find_by_id_for_user");
    let details = details.expect("details");
    assert_eq!(details.tournament_id, tournament_id);
    assert_eq!(details.battletag_id, user_battletag_id);
    assert_eq!(details.alt_accounts, Some(vec!["Alt#1111".to_string()]));
    assert_eq!(details.guarantors, Some(vec!["Guarantor#2222".to_string()]));

    let exists = repo
        .active_registration_exists(tournament_id, 11)
        .await
        .expect("active_registration_exists");
    assert!(exists);
}

#[tokio::test]
async fn registrations_repo_lists_and_actions() {
    let db = TestDb::new().await;
    let repo = RegistrationsRepo::new(db.pool.clone());

    let tournament_id = insert_tournament(
        &db.pool,
        "Action Cup",
        "action-cup",
        "ow2",
        "5v5",
        r#"["2026-01-06"]"#,
        0.0,
        "",
    )
    .await;

    let user_battletag_id = insert_user_with_battletag(&db.pool, 12, "User#4444").await;
    let accepted_id = repo
        .create(build_registration(
            tournament_id,
            12,
            user_battletag_id,
            RegistrationStatus::Accepted,
        ))
        .await
        .expect("create accepted");

    let user_battletag_id2 = insert_user_with_battletag(&db.pool, 13, "User#5555").await;
    repo.create(build_registration(
        tournament_id,
        13,
        user_battletag_id2,
        RegistrationStatus::Declined,
    ))
    .await
    .expect("create declined");

    let accepted = repo
        .list_accepted_for_tournament(tournament_id)
        .await
        .expect("list_accepted_for_tournament");
    assert_eq!(accepted.len(), 1);
    assert_eq!(accepted[0].id, accepted_id);

    sqlx::query("INSERT INTO users (id) VALUES (?1)")
        .bind(99)
        .execute(&db.pool)
        .await
        .expect("insert manager user");

    repo.update_status_with_action(
        accepted_id,
        RegistrationStatus::ActionRequired,
        None,
        Some("Please update info".to_string()),
        99,
    )
    .await
    .expect("update_status_with_action");

    let latest = repo
        .get_latest_pending_action_comment(accepted_id)
        .await
        .expect("get_latest_pending_action_comment");
    assert_eq!(latest.as_deref(), Some("Please update info"));
}

#[tokio::test]
async fn registrations_repo_manager_detail_and_rankings() {
    let db = TestDb::new().await;
    let repo = RegistrationsRepo::new(db.pool.clone());

    let tournament_id = insert_tournament(
        &db.pool,
        "Detail Cup",
        "detail-cup",
        "ow2",
        "5v5",
        r#"["2026-01-07"]"#,
        0.0,
        "",
    )
    .await;

    let user_battletag_id = insert_user_with_battletag(&db.pool, 20, "User#6666").await;
    let registration_id = repo
        .create(build_registration(
            tournament_id,
            20,
            user_battletag_id,
            RegistrationStatus::Pending,
        ))
        .await
        .expect("create registration");

    sqlx::query("INSERT INTO users (id) VALUES (?1)")
        .bind(200)
        .execute(&db.pool)
        .await
        .expect("insert manager user");

    repo.create_comment(registration_id, 200, "first".to_string())
        .await
        .expect("create_comment");

    repo.update_status_with_action(
        registration_id,
        RegistrationStatus::ActionRequired,
        None,
        Some("Need more details".to_string()),
        200,
    )
    .await
    .expect("update_status_with_action");

    let rankings = repo
        .replace_role_rankings(
            registration_id,
            vec![(RoleValue::Tank, 2), (RoleValue::Support, 1)],
        )
        .await
        .expect("replace_role_rankings");
    assert_eq!(rankings.len(), 2);

    let detail = repo
        .get_manager_detail(registration_id)
        .await
        .expect("get_manager_detail")
        .expect("detail");
    assert_eq!(detail.registration.id, registration_id);
    assert_eq!(detail.comments.len(), 1);
    assert_eq!(detail.requested_actions.len(), 1);
    assert_eq!(detail.role_rankings.len(), 2);
}

#[tokio::test]
async fn registrations_repo_resolves_requested_action() {
    let db = TestDb::new().await;
    let repo = RegistrationsRepo::new(db.pool.clone());

    let tournament_id = insert_tournament(
        &db.pool,
        "Resolve Cup",
        "resolve-cup",
        "ow2",
        "5v5",
        r#"["2026-01-08"]"#,
        0.0,
        "",
    )
    .await;

    let user_battletag_id = insert_user_with_battletag(&db.pool, 30, "User#7777").await;
    let registration_id = repo
        .create(build_registration(
            tournament_id,
            30,
            user_battletag_id,
            RegistrationStatus::Pending,
        ))
        .await
        .expect("create registration");

    sqlx::query("INSERT INTO users (id) VALUES (?1)")
        .bind(300)
        .execute(&db.pool)
        .await
        .expect("insert manager user");

    repo.update_status_with_action(
        registration_id,
        RegistrationStatus::ActionRequired,
        None,
        Some("Fix profile".to_string()),
        300,
    )
    .await
    .expect("update_status_with_action");

    let action_id: i64 = sqlx::query_scalar(
        r#"SELECT id FROM registration_requested_actions WHERE registration_id = ?1"#,
    )
    .bind(registration_id)
    .fetch_one(&db.pool)
    .await
    .expect("select action id");

    let resolved = repo
        .resolve_requested_action(registration_id, action_id)
        .await
        .expect("resolve_requested_action");
    let resolved = resolved.expect("resolved action");
    assert_eq!(resolved.status, ActionStatus::Resolved);
}

#[tokio::test]
async fn tournaments_update_changes_title_and_config() {
    let db = TestDb::new().await;
    let tournaments_repo = TournamentsRepo::new(db.pool.clone());

    let id = insert_tournament(
        &db.pool,
        "Старый тайтл",
        "old-sef",
        "OW2",
        "Online",
        r#"["2026-02-21"]"#,
        40000.0,
        "RUB",
    )
    .await;

    tournaments_repo
        .update(
            id,
            UpdateTournamentData {
                title: "Новый тайтл".into(),
                sef_title: "new-sef".into(),
                discipline: "OW2".into(),
                format: "Online".into(),
                dates_json: r#"["2026-03-01"]"#.into(),
                prize_pool_total_amount: 50000.0,
                prize_pool_currency: "RUB".into(),
                configuration_json: r#"{"type":"Online","schedule":[],"prize_pool":{"currency":"RUB","places":[{"place":1,"amount":50000}]}}"#.into(),
            },
        )
        .await
        .expect("update tournament");

    let updated = tournaments_repo
        .get_by_id(id)
        .await
        .expect("get_by_id")
        .expect("updated tournament");
    assert_eq!(updated.title, "Новый тайтл");
    assert_eq!(updated.sef_title, "new-sef");
    assert_eq!(updated.config.prize_pool.currency, Some("RUB".into()));
}

#[tokio::test]
async fn tournaments_update_is_atomic_on_config_failure() {
    let db = TestDb::new().await;
    let tournaments_repo = TournamentsRepo::new(db.pool.clone());

    let id = insert_tournament(
        &db.pool,
        "Atomic title",
        "atomic-sef",
        "OW2",
        "Online",
        r#"["2026-02-21"]"#,
        100.0,
        "RUB",
    )
    .await;

    sqlx::query("DROP TABLE tournament_configuration")
        .execute(&db.pool)
        .await
        .expect("drop tournament_configuration");

    let result = tournaments_repo
        .update(
            id,
            UpdateTournamentData {
                title: "Should rollback".into(),
                sef_title: "should-rollback".into(),
                discipline: "OW2".into(),
                format: "Online".into(),
                dates_json: r#"["2026-03-01"]"#.into(),
                prize_pool_total_amount: 200.0,
                prize_pool_currency: "RUB".into(),
                configuration_json: r#"{"type":"Online","schedule":[],"prize_pool":{}}"#.into(),
            },
        )
        .await;
    assert!(result.is_err());

    let title: String = sqlx::query_scalar("SELECT title FROM tournaments WHERE id = ?1")
        .bind(id)
        .fetch_one(&db.pool)
        .await
        .expect("select title");
    assert_eq!(title, "Atomic title");
}

#[tokio::test]
async fn tournaments_update_upserts_config_when_missing() {
    let db = TestDb::new().await;
    let tournaments_repo = TournamentsRepo::new(db.pool.clone());

    let id = insert_tournament_without_config(
        &db.pool,
        "No config",
        "no-config",
        "OW2",
        "Online",
        r#"["2026-02-21"]"#,
        0.0,
        "",
    )
    .await;

    tournaments_repo
        .update(
            id,
            UpdateTournamentData {
                title: "Config created".into(),
                sef_title: "config-created".into(),
                discipline: "OW2".into(),
                format: "Online".into(),
                dates_json: r#"["2026-03-01"]"#.into(),
                prize_pool_total_amount: 123.0,
                prize_pool_currency: "RUB".into(),
                configuration_json:
                    r#"{"type":"Online","schedule":[],"prize_pool":{"currency":"RUB"}}"#.into(),
            },
        )
        .await
        .expect("update tournament");

    let config: String = sqlx::query_scalar(
        "SELECT configuration_json FROM tournament_configuration WHERE tournament_id = ?1",
    )
    .bind(id)
    .fetch_one(&db.pool)
    .await
    .expect("select config");
    assert!(config.contains("\"type\":\"Online\""));
}
