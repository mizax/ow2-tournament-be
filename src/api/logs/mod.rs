use actix_web::web;

mod handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/logs")
            .app_data(web::PayloadConfig::new(5 * 1024 * 1024))
            .service(handlers::load_log),
    );
}
