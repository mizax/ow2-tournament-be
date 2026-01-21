use std::sync::Arc;
use actix_web::{get, web, HttpResponse, Responder};
use serde::{Serialize};
use crate::api::error::ApiError;
use crate::dal::dal::Dal;

#[derive(Serialize)]
pub struct Tournament<'a> {
    title: &'a str,
    uri: &'a str,
    dates: Vec<&'a str>,
}

#[get("")]
pub async fn get_tournaments(
    dal: web::Data<Arc<Dal>>,
) -> actix_web::Result<impl Responder, ApiError> {
    Ok(HttpResponse::Ok().json(vec![ Tournament {
        title: "День Защитника Пейлоада",
        uri: "2026-02-defender-of-the-payload-day",
        dates: vec!["2026-02-21", "2026-02-22"]
    }]))
}
