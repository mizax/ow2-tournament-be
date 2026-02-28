use actix_web::web;

mod audit;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/admin").configure(audit::configure));
}
