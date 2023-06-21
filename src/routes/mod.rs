use std::fmt::{Debug};
use actix_web::http::StatusCode;
use actix_web::{ResponseError};
use crate::task::retrieve_jar::JarError;

pub mod v0;

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
	#[error("Database Error: {0}")]
	Sqlx(#[from] sqlx::Error),
	#[error("Ferinth Error: {0}")]
	Ferinth(#[from] ferinth::Error),
	#[error("{0}")]
	JarError(#[from] JarError),
	#[error("I/O Error: {0}")]
	Io(#[from] std::io::Error),
	#[error("RusTLS Error: {0}")]
	RusTLS(#[from] rustls::Error),
	#[error("Other: {0}")]
	Other(String),
}

impl ResponseError for ApiError {
	fn status_code(&self) -> StatusCode {
		match self {
			ApiError::Sqlx(..) => StatusCode::BAD_GATEWAY,
			ApiError::Ferinth(..) => StatusCode::INTERNAL_SERVER_ERROR,
			ApiError::JarError(..) => StatusCode::INTERNAL_SERVER_ERROR,
			ApiError::Io(..) => StatusCode::INTERNAL_SERVER_ERROR,
			ApiError::RusTLS(..) => StatusCode::INTERNAL_SERVER_ERROR,
			ApiError::Other(..) => StatusCode::INTERNAL_SERVER_ERROR,
		}
	}
}
