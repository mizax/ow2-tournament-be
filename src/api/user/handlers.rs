use actix_web::{get, HttpResponse, Responder};
use serde_json::json;

#[get("/whoami")]
async fn whoami() -> actix_web::Result<impl Responder> {
    Ok(HttpResponse::Ok().json(json!({})))
}
