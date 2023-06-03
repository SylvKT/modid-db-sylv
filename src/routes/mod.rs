use std::fmt::{Debug, Display, Formatter};
use actix_web::http::StatusCode;
use actix_web::{ResponseError};

pub mod v1;

#[derive(Clone, Debug)]
pub enum ApiError {
	Other(String),
}

impl Display for ApiError {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		Debug::fmt(self, f)
	}
}

impl ResponseError for ApiError {
	fn status_code(&self) -> StatusCode {
		match self {
			ApiError::Other(..) => StatusCode::INTERNAL_SERVER_ERROR,
		}
	}
}
