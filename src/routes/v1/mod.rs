use actix_web::{get, HttpResponse};
use serde::{Deserialize, Serialize};
use crate::routes::ApiError;

pub mod mods;

pub fn config(cfg: &mut actix_web::web::ServiceConfig) {
	cfg.service(
		actix_web::web::scope("v1")
			.service(default)
			.configure(mods::config)
	);
}

// not copied from Labrinth i swear
#[derive(Debug, Serialize, Deserialize)]
struct DefaultRes {
	about: String,
	name: String,
	version: String,
}

#[get("/")]
async fn default() -> Result<HttpResponse, ApiError> {
	let res = HttpResponse::Ok()
		.json(DefaultRes {
			about: String::from("Hello world!"),
			name: String::from("sylv-api"),
			version: String::from("0.1.0"),
		});
	Ok(res)
}
