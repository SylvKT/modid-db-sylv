use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use async_zip::base::read::seek::ZipFileReader;
use ferinth::Ferinth;
use ferinth::structures::{ID, Number};
use ferinth::structures::project::{Project, ProjectType};
use ferinth::structures::search::{Facet, Response, Sort};
use ferinth::structures::version::{Version, VersionFile};
use once_cell::sync::Lazy;
use serde::Deserialize;
use sqlx::{PgPool, query, query_as};
use time::OffsetDateTime;
use tokio::io::AsyncWriteExt;

use crate::util::VariantName;
use crate::routes;
use crate::routes::v0::mods::Platform;

const ALLOWED_LOADERS: &[&str; 2] = &["quilt", "fabric"];

pub static FACETS: Lazy<Vec<Vec<Facet>>> = Lazy::new(|| {
	let mut facets: Vec<Vec<Facet>> = vec![];
	let mut loader_facets: Vec<Facet> = vec![];
	
	for loader in ALLOWED_LOADERS {
		loader_facets.push(Facet::Categories(String::from(*loader)));
	}
	facets.push(loader_facets);
	facets.push(vec![Facet::ProjectType(ProjectType::Mod)]);
	facets
});

#[derive(Debug, thiserror::Error)]
/// An error triggered when handling .jar files.
pub enum JarError {
	#[error("Zip Error: {0}")]
	Zip(#[from] async_zip::error::ZipError),
	#[error("I/O Error: {0}")]
	Io(#[from] std::io::Error),
	#[error("HTTP Request Error: {0}")]
	Http(#[from] reqwest::Error),
	#[error("Ferinth Error: {0}")]
	Ferinth(#[from] ferinth::Error),
	#[error("SQL Database Error: {0}")]
	Sqlx(#[from] sqlx::Error),
	#[error("Compatibility Error: {0}")]
	Compat(#[from] CompatError),
}

impl VariantName for JarError {
	fn variant_name(&self) -> &'static str {
		match self {
			JarError::Zip(..) => "zip",
			JarError::Io(..) => "io",
			JarError::Http(..) => "http",
			JarError::Ferinth(..) => "ferinth",
			JarError::Sqlx(..) => "sqlx",
			JarError::Compat(..) => "compat",
		}
	}
}

#[derive(Debug, thiserror::Error)]
/// An issue with compatibility
pub enum CompatError {
	#[error("Incompatible with mod loader(s): {0}")]
	Loader(String),
}

#[actix_web::main]
pub async fn jar_loop(pool: PgPool) {
	println!("Began Jar Retrieval Loop");
	let mut interval = tokio::time::interval(Duration::from_secs(30 * 60));
	loop {
		if OffsetDateTime::from(SystemTime::now()).minute() % 30 == 0 {
			break;
		}
	}
	
	loop {
		interval.tick().await;
		println!("Checking latest .jar's...");
		match get_fucking_jars(&pool).await {
			Ok(_) => (),
			Err(err) => eprintln!("{}", err),
		}
	}
}

pub async fn download_file_from_ver(ver: Version) -> Result<PathBuf, JarError> {
	// i like boys
	let mut primary: Option<&VersionFile> = None;
	for ver_file in &ver.files {
		if ver_file.primary {
			primary = Some(ver_file);
			break
		}
	}
	
	let ver_file = primary.unwrap_or(ver.files.first().unwrap()); // use first because i'm lazy
	let res = reqwest::get(ver_file.url.clone() /* i'm so sorry, ferris */).await?;
	let mut file = tokio::fs::File::create(&ver_file.filename).await?;
	let bytes = res.bytes().await?;
	let buf: Vec<u8> = bytes.to_vec();
	file.write_all(buf.as_slice()).await?;
	Ok(Path::new(ver_file.filename.as_str()).to_path_buf())
}

pub async fn get_latest_jar(fer: &Ferinth, project_id: &ID) -> Result<(Project, PathBuf), JarError> {
	let mut project = fer.get_project(&*project_id).await?;
	println!("Downloading jar for {} ({}/{})...", project.title, project.slug, project.id);
	
	let mut hit_version_file: Result<Option<PathBuf>, CompatError> = Ok(None);
	// reverse list because the last version returned by labrinth is actually the latest version
	project.versions.reverse();
	for ver_id in project.versions {
		let ver = fer.get_version(&*ver_id).await?;
		if !ver.loaders.iter().any(|x| ALLOWED_LOADERS.contains(&x.as_str())) {
			hit_version_file = Err(CompatError::Loader(format!("{:?}", ver.loaders)));
			continue
		}
		let downloaded_file = download_file_from_ver(ver).await?;
		hit_version_file = Ok(Some(downloaded_file));
		// we downloaded the file! yay!!
		println!("Successfully downloaded .jar for {} ({}/{})!", project.title, project.slug, project.id);
		break // ;-; haha just like me fr r/im14andthisisdeep r/ihavereddit
	}
	
	let project = fer.get_project(&*project_id).await?;
	Ok((project, hit_version_file?.unwrap()))
}

pub async fn get_id_from_jar(path: PathBuf) -> Result<(String, Vec<String>), JarError> {
	// Retrieve the mod ID from the fabric.mod.json or quilt.mod.json
	let ret: Result<(String, Vec<String>), JarError> = { // i'm documenting this code for your dumb ass because i know you'll forget about it -- thanks <3
		let mut id = String::new(); // we return this later; this is the id
		let mut provided = vec![]; // we return this later; these are the provided IDs
		// get id from latest version
		let mut file = tokio::fs::File::open(path.clone()).await?;
		// open zip reader (with tokio)
		let mut zip_reader = ZipFileReader::with_tokio(&mut file).await?;
		let zip = zip_reader.file();
		for index in 0..zip.entries().len() { // iterate over our zip entries old-school
			// open file reader
			let mut reader = zip_reader.reader_with_entry(index).await?;
			let entry = reader.entry();
			let filename = entry.filename().clone();
			if !filename.as_str()?.ends_with(".mod.json") {
				continue;
			}
			// read the file
			let mut string = String::new();
			reader.read_to_string_checked(&mut string).await?;
			match filename.as_str()? {
				"fabric.mod.json" => {
					#[derive(Deserialize)]
					struct Fmj {
						id: String,
						provides: HashMap<String, String>,
					}
					let fmj: Fmj = serde_json::from_str(string.as_ref())
						.expect("Failed to deserialize quilt.mod.json");
					id.push_str(fmj.id.as_str());
					for (id, _) in fmj.provides {
						provided.push(id)
					}
				},
				"quilt.mod.json" => {
					#[derive(Deserialize)]
					struct ProvidesObject {
						id: String,
					}
					#[derive(Deserialize)]
					struct Ql {
						id: String,
						provides: Vec<ProvidesObject>,
					}
					#[derive(Deserialize)]
					struct Qmj {
						quilt_loader: Ql,
					}
					let qmj: Qmj = serde_json::from_str(string.as_ref())
						.expect("Failed to deserialize quilt.mod.json");
					id.push_str(qmj.quilt_loader.id.as_str());
					for provides in qmj.quilt_loader.provides {
						provided.push(provides.id)
					}
				},
				_ => {}
			}
		}
		if id.is_empty() {
			println!("Mod has no fabric.mod.json or quilt.mod.json");
		}
		tokio::fs::remove_file(&*path).await?;
		Ok((id, provided))
	};
	
	Ok(ret?)
}

pub async fn get_projects_and_ids(res: &Response, fer: &Ferinth, projects: &mut Vec<(Project, (String, Vec<String>))>) -> Result<(), JarError> {
	for hit in res.hits.iter() {
		let (project, path) = get_latest_jar(&fer, &hit.project_id).await?;
		let project_result = get_id_from_jar(path).await;
		
		if project_result.is_err() {
			// check if this is a compat or EOCDR error
			let err = project_result.err().unwrap();
			if match err {
				JarError::Compat(_) => true,
				JarError::Zip(async_zip::error::ZipError::UnableToLocateEOCDR) => true,
				_ => false,
			} { // skip this project
				eprintln!("{}", err);
				continue;
			} else { // this is a normal error; return it
				return Err(err)
			}
		}
		
		projects.push((project, project_result.unwrap()));
	}
	Ok(())
}

pub async fn get_fucking_jars(pool: &PgPool) -> Result<(), JarError> {
	// slice-ify facets
	let facets: Vec<&[Facet]> = FACETS.iter().map(|term| term.as_slice()).collect();
	
	let fer = Ferinth::new("SylvKT@github.com/modid-db-sylv", None, Some("mailto:contact@sylv.gay") /* just in case they didn't get the memo */, None)?;
	
	// Request each project's latest .jar
	let mut projects: Vec<(Project, (String, Vec<String>))> = vec![];
	
	// Request newest projects
	let res = fer.search("", &Sort::Newest, facets.as_slice()).await?;
	
	get_projects_and_ids(&res, &fer, &mut projects).await?;
	
	println!("Downloading top 30 newly updated mods.");
	
	// Request newly updated projects
	let res = fer.search_paged("", &Sort::Updated, &Number::from(30usize), &Number::from(0usize), facets.as_slice()).await?;
	
	// second chance
	get_projects_and_ids(&res, &fer, &mut projects).await?;
	
	for (project, ids) in projects {
		let id = ids.0;
		let provides = ids.1;
		// query database with project id
		let mod_opt = query_as!(
			routes::v0::mods::Mod,
			r#"SELECT id, project_id, platform as "platform: _", provides FROM mods WHERE project_id = $1"#,
			project.id.to_string()
		)
			.fetch_optional(&*pool).await?;
		if let Some(r#mod) = mod_opt { // if the mod exists in the database
			if r#mod.id != id { // if this mod ID is new (doesn't match)
				// update the mod ID
				query!(
					"UPDATE mods SET id = $1, provides = $3 WHERE project_id = $2",
					id,
					project.id.to_string(),
					provides.as_slice(),
				)
					.execute(&*pool).await?;
			}
		} else {
			// add mod to database
			query!(
				r#"INSERT INTO mods (id, project_id, platform) VALUES ($1, $2, $3)"#,
				id,
				project.id.to_string(),
				Platform::Modrinth as Platform,
			)
				.execute(&*pool).await?;
			if !provides.is_empty() {
				query!(
					r#"UPDATE mods SET provides = $1 WHERE project_id = $2"#,
					provides.as_slice(),
					project.id.to_string(),
				)
					.execute(&*pool).await?;
			}
		}
	}
	
	Ok(())
}
