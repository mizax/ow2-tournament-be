use actix_web::web;

mod handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/logs").service(handlers::load_log));
}
