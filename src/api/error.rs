use crate::error::AppError;
use actix_web::http::StatusCode;
use log::error;
use actix_web::http::header::ContentType;
use actix_web::{HttpResponse, error};
use thiserror::Error as ThisError;

#[derive(Debug, ThisError)]
pub enum ApiError {
    #[error("An internal error occurred: {error}")]
    InternalError { error: String },
    #[error("Bad request: {error}")]
    BadRequest { error: String, details: String },
    #[error("Conflict: {error}")]
    Conflict { error: String, details: String },
    #[error("Validation error")]
    ValidationError { errors: Vec<String> },
    #[error("Forbidden")]
    Forbidden,
    #[error("Not found")]
    NotFound,
}

impl error::ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match *self {
            ApiError::InternalError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            ApiError::Conflict { .. } => StatusCode::CONFLICT,
            ApiError::ValidationError { .. } => StatusCode::BAD_REQUEST,
            ApiError::Forbidden => StatusCode::FORBIDDEN,
            ApiError::NotFound => StatusCode::NOT_FOUND,
        }
    }

    fn error_response(&self) -> HttpResponse {
        match self {
            ApiError::InternalError { error } => HttpResponse::build(self.status_code())
                .insert_header(ContentType::json())
                .json(serde_json::json!({
                    "error": error,
                })),
            ApiError::BadRequest { error, details } => HttpResponse::build(self.status_code())
                .insert_header(ContentType::json())
                .json(serde_json::json!({
                    "error": error,
                    "details": details,
                })),
            ApiError::Conflict { error, details } => HttpResponse::build(self.status_code())
                .insert_header(ContentType::json())
                .json(serde_json::json!({
                    "error": error,
                    "details": details,
                })),
            ApiError::ValidationError { errors } => HttpResponse::build(self.status_code())
                .insert_header(ContentType::json())
                .json(serde_json::json!({
                    "error": "validation_error",
                    "errors": errors,
                })),
            ApiError::Forbidden => HttpResponse::build(self.status_code())
                .insert_header(ContentType::json())
                .json(serde_json::json!({
                    "error": "forbidden",
                })),
            ApiError::NotFound => HttpResponse::build(self.status_code())
                .insert_header(ContentType::json())
                .json(serde_json::json!({
                    "error": "Not Found",
                })),
        }
    }
}

impl From<AppError> for ApiError {
    fn from(value: AppError) -> Self {
        error!("AppError: {:?}", value);
        ApiError::InternalError {
            error: "Internal error".to_string(),
        }
    }
}
