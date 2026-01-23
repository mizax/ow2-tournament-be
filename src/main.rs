use std::sync::Arc;
use actix_web::{web, App, HttpServer};
use tokio::try_join;
use crate::config::Config;
use crate::dal::dal::Dal;
use crate::storage::create_storage;
use crate::api::auth::state_store::OAuthStateStore;
use crate::api::auth::jwks::BNETJwksService;
use std::time::Duration;

mod config;
mod dal;
mod jobs;
mod api;
mod error;
mod storage;

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
    let jwks_service = web::Data::new(BNETJwksService::new(conf.battlenet.jwks_url.clone(), Duration::from_secs(3600)));

    let job_scheduler =
        jobs::init_scheduler(db.clone());

    let http_server = HttpServer::new(move || {
        App::new()
            .wrap(actix_web::middleware::Logger::new(LOGGER_FORMAT))
            .app_data(web::Data::new(config.clone()))
            .app_data(web::Data::new(db.clone()))
            .app_data(storage.clone())
            .app_data(state_store.clone())
            .app_data(jwks_service.clone())
            // api routes
            .configure(api::configure)
    })
        .workers(conf.actix_workers)
        .bind(conf.server_addr.clone())?
        .run();

    try_join!(
        http_server,
        async { Ok::<(), std::io::Error>(job_scheduler.await.unwrap()) },
    )?;

    Ok(())
}
