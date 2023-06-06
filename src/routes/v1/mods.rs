// maybe naming this "mods" was a bad idea 😅

use actix_web::{get, HttpRequest, HttpResponse, web};
use actix_web::http::StatusCode;
use actix_web::web::ServiceConfig;
use serde::{Serialize, Deserialize};
use sqlx::{PgPool};
use crate::routes::ApiError;

pub fn config(cfg: &mut ServiceConfig) {
	cfg.service(
		web::scope("/mods")
			.service(get_from_id)
	);
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "platform", rename_all = "snake_case")]
pub enum Platform {
	Modrinth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mod {
	id: String,
	name: String,
	description: Option<String>,
	thumbnail: Option<String>,
	project_id: String,
	platform: Platform,
}

// BEGIN TOMFUCKERY

// TODO: fuck
impl ::sqlx::encode::Encode<'_, sqlx::Postgres> for Mod
	where
		String: for<'q> ::sqlx::encode::Encode<'q, sqlx::Postgres>,
		String: ::sqlx::types::Type<sqlx::Postgres>,
		String: for<'q> ::sqlx::encode::Encode<'q, sqlx::Postgres>,
		String: ::sqlx::types::Type<sqlx::Postgres>,
		Option<String>: for<'q> ::sqlx::encode::Encode<'q, sqlx::Postgres>,
		Option<String>: ::sqlx::types::Type<sqlx::Postgres>,
		Option<String>: for<'q> ::sqlx::encode::Encode<'q, sqlx::Postgres>,
		Option<String>: ::sqlx::types::Type<sqlx::Postgres>,
		String: for<'q> ::sqlx::encode::Encode<'q, sqlx::Postgres>,
		String: ::sqlx::types::Type<sqlx::Postgres>,
		Platform: for<'q> ::sqlx::encode::Encode<'q, sqlx::Postgres>,
		Platform: ::sqlx::types::Type<sqlx::Postgres>,
{
	fn encode_by_ref(
		&self,
		buf: &mut sqlx::postgres::PgArgumentBuffer,
	) -> sqlx::encode::IsNull {
		let mut encoder = sqlx::postgres::types::PgRecordEncoder::new(buf);
		encoder.encode(&self.id);
		encoder.encode(&self.name);
		encoder.encode(&self.description);
		encoder.encode(&self.thumbnail);
		encoder.encode(&self.project_id);
		encoder.encode(&self.platform);
		encoder.finish();
		sqlx::encode::IsNull::No
	}
	fn size_hint(&self) -> ::std::primitive::usize {
		6usize * (4 + 4)
			+ <String as ::sqlx::encode::Encode<
			sqlx::Postgres,
		>>::size_hint(&self.id)
			+ <String as ::sqlx::encode::Encode<
			sqlx::Postgres,
		>>::size_hint(&self.name)
			+ <Option<
			String,
		> as ::sqlx::encode::Encode<
			sqlx::Postgres,
		>>::size_hint(&self.description)
			+ <Option<
			String,
		> as ::sqlx::encode::Encode<
			sqlx::Postgres,
		>>::size_hint(&self.thumbnail)
			+ <String as ::sqlx::encode::Encode<
			sqlx::Postgres,
		>>::size_hint(&self.project_id)
			+ <Platform as ::sqlx::encode::Encode<
			sqlx::Postgres,
		>>::size_hint(&self.platform)
	}
}

// this is where the important part and the reason we're doing this
// see: https://github.com/launchbadge/sqlx/issues/1031
impl<'r> sqlx::decode::Decode<'r, sqlx::Postgres> for Mod
	where
		String: sqlx::types::Type<sqlx::Postgres>,
		String: sqlx::types::Type<sqlx::Postgres>,
		Option<String>: sqlx::decode::Decode<'r, sqlx::Postgres>,
		Option<String>: sqlx::types::Type<sqlx::Postgres>,
		Option<String>: sqlx::decode::Decode<'r, sqlx::Postgres>,
		Option<String>: sqlx::types::Type<sqlx::Postgres>,
		String: sqlx::types::Type<sqlx::Postgres>,
		Platform: sqlx::decode::Decode<'r, sqlx::Postgres>,
		Platform: sqlx::types::Type<sqlx::Postgres>,
{
	fn decode(
		value: sqlx::postgres::PgValueRef<'r>,
	) -> Result<
		Self,
		Box<
			dyn std::error::Error + 'static + Send + Sync,
		>,
	> {
		let mut decoder = sqlx::postgres::types::PgRecordDecoder::new(
			value,
		)?;
		let id = decoder.try_decode::<String>()?;
		let name = decoder.try_decode::<String>()?;
		let description = decoder.try_decode::<Option<String>>()?;
		let thumbnail = decoder.try_decode::<Option<String>>()?;
		let project_id = decoder.try_decode::<String>()?;
		let platform = decoder.try_decode::<Platform>()?;
		Ok(Mod {
			id,
			name,
			description,
			thumbnail,
			project_id,
			platform,
		})
	}
}

impl ::sqlx::Type<sqlx::Postgres> for Mod {
	fn type_info() -> sqlx::postgres::PgTypeInfo {
		sqlx::postgres::PgTypeInfo::with_name("Mod")
	}
}

// END TOMFUCKERY

#[derive(Debug, Deserialize)]
struct IdQuery {
	pub id: String,
}

#[get("/get")]
async fn get_from_id(
	_req: HttpRequest,
	web::Query(id): web::Query<IdQuery>,
	pool: web::Data<PgPool>,
) -> Result<HttpResponse, ApiError> {
	let mods = sqlx::query_as!(
		Mod,
		r#"SELECT id, name, description, thumbnail, project_id, platform as "platform: _" FROM mods WHERE id = $1;"#,
		id.id,
	)
		.fetch_all(&**pool)
		.await?;
	let res = HttpResponse::build(StatusCode::OK)
		.json(mods);
	Ok(res)
}
