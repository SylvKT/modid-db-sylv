use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use async_recursion::async_recursion;
use async_zip::base::read::seek::ZipFileReader;
use async_zip::error::ZipError;
use ferinth::Ferinth;
use ferinth::structures::ID;
use ferinth::structures::project::Project;
use ferinth::structures::search::{Facet, Sort};
use ferinth::structures::version::{Version, VersionFile};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, query, query_as};
use time::OffsetDateTime;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::routes;
use crate::routes::v1::mods::Platform;

const ALLOWED_LOADERS: &[&str; 2] = &["quilt", "fabric"];

#[derive(Debug, thiserror::Error)]
/// An error triggered when
pub enum JarError {
	#[error("Zip Error: {0}")]
	ZipError(#[from] async_zip::error::ZipError),
	#[error("I/O Error: {0}")]
	IoError(#[from] std::io::Error),
	#[error("HTTP Request Error: {0}")]
	HttpError(#[from] reqwest::Error),
	#[error("Ferinth Error: {0}")]
	FerinthError(#[from] ferinth::Error),
	#[error("SQL Database Error: {0}")]
	SqlError(#[from] sqlx::Error),
	#[error("Compatibility Error: {0}")]
	CompatError(#[from] CompatError),
}

#[derive(Debug, thiserror::Error)]
/// An issue with compatibility
pub enum CompatError {
	#[error("Incompatible with mod loader(s): {0}")]
	Loader(String),
}

pub async fn jar_loop(pool: PgPool) {
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
	file.write(buf.as_slice()).await?;
	Ok(Path::new(ver_file.filename.as_str()).to_path_buf())
}

pub async fn get_latest_jar(fer: &Ferinth, project_id: &ID) -> Result<(Project, PathBuf), JarError> {
	let project = fer.get_project(&*project_id).await?;
	println!("Downloading jar for {} ({}/{})...", project.title, project.slug, project.id);
	
	let mut hit_version_file: Result<Option<PathBuf>, CompatError> = Ok(None);
	for ver_id in project.versions {
		let ver = fer.get_version(&ver_id).await?;
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

pub async fn get_id_from_jar(project: Project, path: PathBuf) -> Result<(Project, String), JarError> {
	// Retrieve the mod ID from the fabric.mod.json or quilt.mod.json
	let id: Result<String, JarError> = { // i'm documenting this code for your dumb ass because i know you'll forget about it
		let mut id_ret = String::new(); // we return this later; this is the id
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
					}
					let fmj: Fmj = serde_json::from_str(string.as_ref())
						.expect("Failed to deserialize quilt.mod.json");
					id_ret = fmj.id;
				},
				"quilt.mod.json" => {
					#[derive(Deserialize)]
					struct Ql {
						id: String,
					}
					#[derive(Deserialize)]
					struct Qmj {
						quilt_loader: Ql,
					}
					let qmj: Qmj = serde_json::from_str(string.as_ref())
						.expect("Failed to deserialize quilt.mod.json");
					id_ret = qmj.quilt_loader.id;
				},
				_ => {}
			}
		}
		if id_ret.is_empty() {
			println!("Mod has no fabric.mod.json or quilt.mod.json");
			println!("This should not happen unless there is a fork of Fabric or Quilt!");
		}
		tokio::fs::remove_file(&*path).await?;
		Ok(id_ret)
	};
	
	Ok((project, id?))
}

#[async_recursion]
// cursed attempt to fix the EOCDR error
// Don't ask why, but this works
pub async fn attempt_get_id_from_jar(project: Project, path: PathBuf, attempt: u16) -> Result<(Project, String), JarError> {
	let project_result = get_id_from_jar(project.clone(), path.clone()).await;
	return if project_result.is_err() {
		// check if this is a compat or EOCDR error
		let err = project_result.err().unwrap();
		return if match err {
			JarError::CompatError(_) => true,
			JarError::ZipError(ZipError::UnableToLocateEOCDR) => true,
			_ => false,
		} { // attempt again
			eprintln!("{}", err);
			eprintln!("Attempt #{}", attempt);
			if attempt > 0 {
				attempt_get_id_from_jar(project, path, attempt - 1).await
			} else {
				Err(err)
			}
		} else { // this is a normal error; return it
			Err(err)
		}
	} else {
		project_result
	}
}

pub async fn get_fucking_jars(pool: &PgPool) -> Result<(), JarError> {
	let mut facets: Vec<Vec<Facet>> = vec![];
	
	for loader in ALLOWED_LOADERS {
		facets.push(vec![Facet::Categories(String::from(*loader))]);
	}
	let facets: Vec<&[Facet]> = facets.iter().map(|term| term.as_slice()).collect();
	
	let fer = Ferinth::new("SylvKT@github.com/modid-db-sylv", None, Some("mailto:contact@sylv.gay") /* just in case they didn't get the memo */, None)?;
	
	// Request newest projects
	let res = fer.search("", &Sort::Newest, facets.as_slice()).await?;
	
	// Request each project's latest .jar
	let mut projects: Vec<(Project, String)> = vec![];
	for hit in res.hits {
		let (project, path) = get_latest_jar(&fer, &hit.project_id).await?;
		let project_result = attempt_get_id_from_jar(project, path, 5).await;
		
		if project_result.is_err() {
			// check if this is a compat or EOCDR error
			let err = project_result.err().unwrap();
			if match err {
				JarError::CompatError(_) => true,
				JarError::ZipError(ZipError::UnableToLocateEOCDR) => true,
				_ => false,
			} { // skip this project
				eprintln!("{}", err);
				continue;
			} else { // this is a normal error; return it
				return Err(err)
			}
		}
		
		projects.push(project_result.unwrap());
	}
	
	for (project, id) in projects {
		// query database with project id
		let mod_opt = query_as!(
			routes::v1::mods::Mod,
			r#"SELECT id, name, description, thumbnail, project_id, platform as "platform: _" FROM mods WHERE project_id = $1"#,
			project.id.to_string()
		)
			.fetch_optional(&*pool).await?;
		if let Some(r#mod) = mod_opt { // if the mod exists in the database
			if r#mod.id != id { // if this mod ID is new (doesn't match)
				// update the mod ID
				query!(
					"UPDATE mods SET id = $1 WHERE project_id = $2",
					id,
					project.id.to_string()
				)
					.execute(&*pool).await?;
			}
			// if name doesn't match
			// if description doesn't match
			// if thumbnail doesn't match
		} else {
			// add mod to database
			let mut icon_url: Option<String> = None;
			if project.icon_url.is_some() {
				icon_url = Some(project.icon_url.unwrap().to_string());
			}
			query!(
				r#"INSERT INTO mods (id, name, description, thumbnail, project_id, platform) VALUES ($1, $2, $3, $4, $5, $6)"#,
				id,
				project.title,
				project.description,
				icon_url,
				project.id.to_string(),
				Platform::Modrinth as Platform
			)
				.execute(&*pool).await?;
		}
	}
	Ok(())
}
