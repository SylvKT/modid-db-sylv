use actix_web::{get, HttpRequest, HttpResponse, web};
use actix_web::web::ServiceConfig;
use serde::Deserialize;
use sqlx::{Executor, PgPool, query};
use crate::routes::ApiError;

pub fn config(cfg: &mut ServiceConfig) {
	cfg.service(
		web::scope("/mod-db")
			.service(get_from_id)
	);
}

#[derive(Debug, Deserialize)]
struct IdQuery {
	pub id: String,
}

#[get("/get")]
async fn get_from_id(
	req: HttpRequest,
	web::Query(id): web::Query<IdQuery>,
	pool: web::Data<PgPool>,
) -> Result<HttpResponse, ApiError> {
	sqlx::query!(
		"SELECT * FROM ids WHERE $1;",
		id.id,
	);
	Ok()
}
