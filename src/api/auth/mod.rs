use actix_web::web;

mod battlenet;
pub mod jwks;
pub mod jwt;
pub mod state_store;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/auth")
            .service(battlenet::auth)
            .service(battlenet::callback),
    );
}
