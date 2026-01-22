use crate::api::auth::jwt::AuthenticatedUser;
use actix_web::{get, HttpResponse, Responder};

#[get("/whoami")]
async fn whoami(user: AuthenticatedUser) -> actix_web::Result<impl Responder> {
    Ok(HttpResponse::Ok().json(user))
}
