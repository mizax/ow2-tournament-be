use actix_web::web;
mod handlers;
mod models;
mod secured_handlers;

pub fn configure_public(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/tournaments")
            .service(handlers::get_tournaments)
            .service(handlers::get_tournament)
            .service(handlers::get_tournament_registrations),
    );
}

pub fn configure_private(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/tournaments")
            .service(secured_handlers::register)
            .service(secured_handlers::registration_status),
    );
}
