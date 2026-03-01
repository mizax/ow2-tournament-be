use actix_web::{App, HttpResponse, HttpServer, web};
use ow2_tournament_be::api::auth::jwks::BNETJwksService;
use ow2_tournament_be::api::auth::state_store::OAuthStateStore;
use ow2_tournament_be::config::Config;
use ow2_tournament_be::dal::dal::Dal;
use ow2_tournament_be::services::{GeoIpService, TournamentLiveStreamsService, TwitchApiService};
use ow2_tournament_be::storage::create_storage;
use std::sync::Arc;
use std::time::Duration;
use tokio::try_join;

use ow2_tournament_be::api;
use ow2_tournament_be::jobs;

const LOGGER_FORMAT: &'static str = "%{r}a \"%r\" %s %b \"%{Referer}i\" \"%{User-Agent}i\" %T";

fn main() {
    pretty_env_logger::init_timed();
    dotenv::dotenv().ok();

    let config = match Config::from_env() {
        Ok(c) => c,
        Err(e) => panic!("error creating config: {}", e),
    };

    actix_web::rt::System::with_tokio_rt(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .worker_threads(8)
            .thread_name("main-tokio")
            .build()
            .unwrap()
    })
    .block_on(async_main(&config))
    .unwrap();
}

async fn async_main(conf: &Config) -> std::io::Result<()> {
    let config = Arc::new(conf.clone());

    let db = Arc::new(Dal::new(conf).await);

    let storage = web::Data::new(create_storage(conf.storage.clone()).await.unwrap());

    let state_store = web::Data::new(OAuthStateStore::new(Duration::from_secs(3600)));
    let jwks_service = web::Data::new(BNETJwksService::new(
        conf.battlenet.jwks_url.clone(),
        Duration::from_secs(3600),
    ));
    let geo_ip_service = web::Data::new(GeoIpService::new(conf.geoip_enabled));
    let twitch_api_service = match (
        conf.twitch.client_id.as_deref().map(str::trim),
        conf.twitch.client_secret.as_deref().map(str::trim),
    ) {
        (Some(client_id), Some(client_secret))
            if !client_id.is_empty() && !client_secret.is_empty() =>
        {
            Some(Arc::new(TwitchApiService::new(
                client_id.to_string(),
                client_secret.to_string(),
            )))
        }
        _ => {
            log::warn!(
                "TWITCH.CLIENT_ID/TWITCH.CLIENT_SECRET are not configured. Live-streams service will be disabled."
            );
            None
        }
    };
    let twitch_live_streams_service = twitch_api_service
        .as_ref()
        .map(|svc| web::Data::new(TournamentLiveStreamsService::new(svc.clone())));

    let job_scheduler = jobs::init_scheduler(db.clone());

    let http_client = web::Data::new(reqwest::Client::new());

    let http_server = HttpServer::new(move || {
        let json_cfg = web::JsonConfig::default().error_handler(|err, _req| {
            let message = err.to_string();
            let response = HttpResponse::BadRequest()
                .content_type(actix_web::http::header::ContentType::json())
                .json(serde_json::json!({
                    "error": "invalid_request",
                    "details": message,
                }));
            actix_web::error::InternalError::from_response(err, response).into()
        });

        let app = App::new()
            .wrap(actix_web::middleware::Logger::new(LOGGER_FORMAT))
            .app_data(json_cfg)
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(db.clone()))
            .app_data(storage.clone())
            .app_data(state_store.clone())
            .app_data(jwks_service.clone())
            .app_data(geo_ip_service.clone())
            .app_data(http_client.clone());

        let app = match (
            twitch_api_service.as_ref(),
            twitch_live_streams_service.as_ref(),
        ) {
            (Some(twitch_api_service), Some(twitch_live_streams_service)) => app
                .app_data(web::Data::from(twitch_api_service.clone()))
                .app_data(twitch_live_streams_service.clone()),
            _ => app,
        };

        app.configure(api::configure)
    })
    .workers(conf.actix_workers)
    .bind(conf.server_addr.clone())?
    .run();

    try_join!(http_server, async {
        Ok::<(), std::io::Error>(job_scheduler.await.unwrap())
    },)?;

    Ok(())
}
