mod error;
pub mod auth;

use actix_web::{get, web, HttpResponse, Result, Responder};
use serde_json::json;

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
            .service(stub)
            .configure(auth::battlenet::configure)
    );
}

fn configure_private(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/secured/v1")
            .service(stub)
    );
}

#[get("/")]
async fn stub() -> Result<impl Responder> {
    Ok(HttpResponse::Ok().json(json!({})))
}
