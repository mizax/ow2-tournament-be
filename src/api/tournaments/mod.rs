use actix_web::web;
mod handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/tournaments")
            .service(handlers::get_tournaments)
            .service(handlers::get_tournament)
    );
}
