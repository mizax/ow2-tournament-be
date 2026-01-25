mod error;
pub mod auth;
mod tournaments;
mod user;
mod logs;
mod registrations;
mod manager;

use actix_web::{web};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .configure(configure_public)
            .configure(configure_private)
    );
}

fn configure_public(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/public/v1")
            .configure(auth::configure)
            .configure(tournaments::configure_public)
    );
}

fn configure_private(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/secured/v1")
            .configure(user::configure)
            .configure(logs::configure)
            .configure(tournaments::configure_private)
            .configure(registrations::configure)
            .configure(manager::configure)
    );
}
