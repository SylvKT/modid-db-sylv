// maybe naming this "mods" was a bad idea 😅

use actix_web::{get, HttpRequest, HttpResponse, web};
use actix_web::http::StatusCode;
use actix_web::web::ServiceConfig;
use ferinth::Ferinth;
use ferinth::structures::{ID, Number};
use ferinth::structures::project::Project;
use ferinth::structures::search::Sort;
use serde::{Serialize, Deserialize};
use sqlx::{PgPool};
use crate::routes::ApiError;
use crate::task::retrieve_jar::{default_facets, get_id_from_jar, get_latest_jar, get_projects_and_ids};

pub fn config(cfg: &mut ServiceConfig) {
	cfg.service(
		web::scope("/mods")
			.service(get_from_id)
			.service(get_from_project_id)
	);
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "platform", rename_all = "snake_case")]
pub enum Platform {
	Modrinth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mod {
	pub id: String,
	pub project_id: String,
	pub platform: Platform,
}

// BEGIN TOMFUCKERY

// TODO: fuck
impl ::sqlx::encode::Encode<'_, sqlx::Postgres> for Mod
	where
		String: for<'q> ::sqlx::encode::Encode<'q, sqlx::Postgres>,
		String: ::sqlx::types::Type<sqlx::Postgres>,
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
		String: sqlx::types::Type<sqlx::Postgres>,
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
		let project_id = decoder.try_decode::<String>()?;
		let platform = decoder.try_decode::<Platform>()?;
		Ok(Mod {
			id,
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

#[derive(Debug, Serialize, Deserialize)]
struct IdQuery {
	pub id: String,
}

/// This queries the database for the given project ID and either adds it or updates it depending on if it exists
async fn set_or_update_mod(project: &Project, id: String, pool: &PgPool) -> Result<HttpResponse, ApiError> {
	if id.len() == 0 {
		return Err(ApiError::Other("Failed to extract mod ID".to_string()))
	}
	// query database with project id
	let mod_opt = sqlx::query_as!(
				Mod,
				r#"SELECT id, project_id, platform as "platform: _" FROM mods WHERE project_id = $1"#,
				project.id.to_string()
			)
		.fetch_optional(&*pool).await?;
	if let Some(r#mod) = mod_opt { // if the mod exists in the database
		if r#mod.id != id { // if this mod ID is new (doesn't match)
			// update the mod ID
			sqlx::query!(
						"UPDATE mods SET id = $1 WHERE project_id = $2",
						id,
						project.id.to_string()
					)
				.execute(&*pool).await?;
		}
	} else {
		// add mod to database
		sqlx::query!(
					r#"INSERT INTO mods (id, project_id, platform) VALUES ($1, $2, $3)"#,
					id,
					project.id.to_string(),
					Platform::Modrinth as Platform
				)
			.execute(&*pool).await?;
	}
	
	let r#mod = Mod {
		id,
		project_id: project.id.clone(),
		platform: Platform::Modrinth,
	};
	let res = HttpResponse::Ok()
		.json(r#mod);
	Ok(res)
}

#[get("/get")]
async fn get_from_id(
	web::Query(query): web::Query<IdQuery>,
	pool: web::Data<PgPool>,
	fer: web::Data<Ferinth>,
) -> Result<HttpResponse, ApiError> {
	let mods = sqlx::query_as!(
		Mod,
		r#"SELECT id, project_id, platform as "platform: _" FROM mods WHERE id = $1;"#,
		query.id,
	)
		.fetch_all(&**pool)
		.await?;
	
	if mods.is_empty() { // search in the modrinth query
		let max_results = 5usize;
		let facets = default_facets();
		let res = fer.search_paged(&*query.id, &Sort::Relevance, &Number::from(max_results), &Number::from(0usize), facets.as_ref()).await?;
		let mut projects = vec![];
		get_projects_and_ids(&res, &fer, &mut projects).await?;
		for proj_id in projects {
			if proj_id.1 == query.id {
				let project = proj_id.0;
				let id = proj_id.1;
				return set_or_update_mod(&project, id, pool.get_ref()).await
			}
		}
	}
	
	let res = HttpResponse::build(StatusCode::OK)
		.json(mods);
	Ok(res)
}

#[get("/{project_id}")]
async fn get_from_project_id(
	path: web::Path<ID>,
	pool: web::Data<PgPool>,
	fer: web::Data<Ferinth>,
) -> Result<HttpResponse, ApiError> {
	let mods = sqlx::query_as!(
		Mod,
		r#"SELECT id, project_id, platform as "platform: _" FROM mods WHERE project_id = $1;"#,
		path.clone(),
	)
		.fetch_one(&**pool)
		.await;
	if mods.is_err() {
		let err = mods.err().unwrap();
		return if match err {
			sqlx::Error::RowNotFound => true,
			_ => false,
		} { // download mod and add id to database
			let (project, path) = get_latest_jar(fer.as_ref(), &*path).await?;
			let id = get_id_from_jar(path).await?;
			set_or_update_mod(&project, id, pool.get_ref()).await
		} else {
			Err(ApiError::from(err))
		}
	}
	let res = HttpResponse::Ok()
		.json(mods.unwrap());
	Ok(res)
}
