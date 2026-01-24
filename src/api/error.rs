use actix_web::{error, HttpResponse};
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use thiserror::Error as ThisError;
use crate::error::AppError;

#[derive(Debug, ThisError)]
pub enum ApiError {
    #[error("An internal error occurred: {error}")]
    InternalError { error: String },
    #[error("Bad request: {error}")]
    BadRequest { error: String, details: String },
    #[error("Validation error")]
    ValidationError { errors: Vec<String> },
    #[error("Not found")]
    NotFound,
}

impl error::ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match *self {
            ApiError::InternalError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::BadRequest { .. } => StatusCode::BAD_REQUEST,
            ApiError::ValidationError { .. } => StatusCode::BAD_REQUEST,
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
            ApiError::ValidationError { errors } => HttpResponse::build(self.status_code())
                .insert_header(ContentType::json())
                .json(serde_json::json!({
                        "errors": errors,
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
        match value {
            _ => ApiError::InternalError { error: "Internal error".to_string() },
        }
    }
}
