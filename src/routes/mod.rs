use std::fmt::{Debug};
use actix_web::http::StatusCode;
use actix_web::{ResponseError};
use crate::task::retrieve_jar::JarError;

pub mod v1;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
	#[error("Database Error: {0}")]
	Sqlx(#[from] sqlx::Error),
	#[error("{0}")]
	JarError(#[from] JarError),
	#[error("Other: {0}")]
	Other(String),
}

impl ResponseError for ApiError {
	fn status_code(&self) -> StatusCode {
		match self {
			ApiError::Sqlx(..) => StatusCode::INTERNAL_SERVER_ERROR,
			ApiError::JarError(..) => StatusCode::INTERNAL_SERVER_ERROR,
			ApiError::Other(..) => StatusCode::INTERNAL_SERVER_ERROR,
		}
	}
}
