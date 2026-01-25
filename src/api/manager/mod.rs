use actix_web::web;

mod registrations;
mod tournaments;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/manager")
            .configure(registrations::configure)
            .configure(tournaments::configure),
    );
}
