use actix_web::dev::ServiceResponse;
use crate::routes::ApiError;
use actix_web::{Result, ResponseError};
use actix_web::middleware::ErrorHandlerResponse;

pub fn handle_400<B>(res: ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
	Ok(ErrorHandlerResponse::Response(res.map_body(|res_head, _| {
		ApiError::BadRequest(res_head.reason().to_string()).error_response().into_body()
	}).map_into_right_body()))
}

pub fn handle_404<B>(res: ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
	Ok(ErrorHandlerResponse::Response(res.map_body(|res_head, _| {
		ApiError::NotFound(res_head.reason().to_string()).error_response().into_body()
	}).map_into_right_body()))
}
