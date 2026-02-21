use crate::storage::StorageConfig;
pub use ::config::ConfigError;
use serde::Deserialize;
use sqlx::sqlite::{SqliteAutoVacuum, SqliteConnectOptions, SqliteJournalMode};

#[derive(Deserialize, Clone)]
pub struct Config {
    pub server_addr: String,
    pub actix_workers: usize,
    pub sqlite: CustomSqliteConnectOptions,
    pub app: App,
    #[serde(default = "default_geoip_enabled")]
    pub geoip_enabled: bool,
    #[serde(default = "default_storage_config")]
    pub storage: StorageConfig,
    pub battlenet: BattleNetConfig,
    #[serde(default)]
    pub twitch: TwitchConfig,
}

#[derive(Deserialize, Clone)]
pub struct BattleNetConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub jwks_url: String,
    #[serde(default)]
    pub token_url: Option<String>,
    #[serde(default)]
    pub userinfo_url: Option<String>,
}

#[derive(Deserialize, Clone, Default)]
pub struct TwitchConfig {
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct App {
    pub main_host: String,
    pub alt_host: String,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let builder = config::Config::builder().add_source(config::Environment::default());
        match builder.build() {
            Ok(cfg) => cfg.try_deserialize(),
            Err(e) => Err(e),
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct CustomSqliteConnectOptions {
    pub filename: String,
    pub text_extension: String,
}

impl TryInto<SqliteConnectOptions> for CustomSqliteConnectOptions {
    type Error = ();

    fn try_into(self) -> Result<SqliteConnectOptions, Self::Error> {
        Ok(SqliteConnectOptions::new()
            .journal_mode(SqliteJournalMode::Wal)
            .auto_vacuum(SqliteAutoVacuum::Incremental)
            .filename(self.filename)
            .extension(self.text_extension))
    }
}

fn default_storage_config() -> StorageConfig {
    StorageConfig::FileSystem {
        root_dir: "./storage".to_string(),
        shard_levels: None,
        shard_chars: None,
        serve_url: None,
    }
}

fn default_geoip_enabled() -> bool {
    true
}
