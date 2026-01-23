use actix_web::web;

pub mod state_store;
pub mod jwt;
pub mod jwks;
mod battlenet;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .service(battlenet::auth)
            .service(battlenet::callback)
    );
}
