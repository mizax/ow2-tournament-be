use actix_web::web;
mod handlers;
mod secured_handlers;
mod models;

pub fn configure_public(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/tournaments")
            .service(handlers::get_tournaments)
            .service(handlers::get_tournament)
    );
}

pub fn configure_private(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/tournaments")
            .service(secured_handlers::register)
            .service(secured_handlers::registration_status)
    );
}
