use actix_web::{error, HttpResponse};
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use thiserror::Error as ThisError;
use crate::error::AppError;

#[derive(Debug, ThisError)]
pub enum ApiError {
    #[error("An internal error occurred: {error}")]
    InternalError { error: String },
}


impl error::ResponseError for ApiError {
    fn status_code(&self) -> StatusCode {
        match *self {
            ApiError::InternalError { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        match self {
            ApiError::InternalError { error } => HttpResponse::build(self.status_code())
                .insert_header(ContentType::json())
                .json(serde_json::json!({
                        "error": error,
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
