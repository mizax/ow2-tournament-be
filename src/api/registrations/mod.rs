use actix_web::web;

mod secured_handlers;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/registrations").service(secured_handlers::get_registration));
}
