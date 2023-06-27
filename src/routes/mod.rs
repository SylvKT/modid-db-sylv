use std::fmt::Debug;

use actix_web::{HttpResponse, ResponseError};
use actix_web::body::BoxBody;
use actix_web::http::StatusCode;
use serde::Serialize;

use crate::util::VariantName;
use crate::task::retrieve_jar::JarError;

pub mod v0;

#[derive(Debug, Serialize)]
struct ApiErrorResponse {
	error_code: u16,
	error_type: String,
	description: String,
}

#[derive(Debug, thiserror::Error)]
/// An error that the database throws.
pub enum ApiError {
	#[error("Database Error: {0}")]
	Sqlx(#[from] sqlx::Error),
	#[error("Ferinth Error: {0}")]
	Ferinth(#[from] ferinth::Error),
	#[error("Jar Error: {0}")]
	JarError(#[from] JarError),
	#[error("I/O Error: {0}")]
	Io(#[from] std::io::Error),
	#[error("RusTLS Error: {0}")]
	RusTLS(#[from] rustls::Error),
	#[error("Bad Request: {0}")]
	BadRequest(String),
	#[error("Not Found: {0}")]
	NotFound(String),
	#[error("Other: {0}")]
	Other(String),
}

impl VariantName for ApiError {
	fn variant_name(&self) -> &'static str {
		match self {
			ApiError::Sqlx(..) => "sqlx",
			ApiError::Ferinth(..) => "ferinth",
			ApiError::JarError(err) => err.variant_name(),
			ApiError::Io(..) => "io",
			ApiError::RusTLS(..) => "rustls",
			ApiError::BadRequest(..) => "bad_request",
			ApiError::NotFound(..) => "not_found",
			ApiError::Other(..) => "other",
		}
	}
}

impl ResponseError for ApiError {
	fn status_code(&self) -> StatusCode {
		match self {
			ApiError::Sqlx(..) => StatusCode::BAD_GATEWAY,
			ApiError::Ferinth(..) => StatusCode::INTERNAL_SERVER_ERROR,
			ApiError::JarError(..) => StatusCode::INTERNAL_SERVER_ERROR,
			ApiError::Io(..) => StatusCode::INTERNAL_SERVER_ERROR,
			ApiError::RusTLS(..) => StatusCode::INTERNAL_SERVER_ERROR,
			ApiError::BadRequest(..) => StatusCode::BAD_REQUEST,
			ApiError::NotFound(..) => StatusCode::NOT_FOUND,
			ApiError::Other(..) => StatusCode::INTERNAL_SERVER_ERROR,
		}
	}
	
	fn error_response(&self) -> HttpResponse<BoxBody> {
		HttpResponse::build(self.status_code()).json(ApiErrorResponse {
			error_code: self.status_code().as_u16(),
			error_type: self.variant_name().to_string(),
			description: self.to_string(),
		})
	}
}
