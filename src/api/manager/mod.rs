use actix_web::web;

mod registrations;
mod tournament_managers;
mod tournaments;
mod users;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/manager")
            .configure(registrations::configure)
            .configure(tournaments::configure)
            .configure(users::configure),
    );
}
