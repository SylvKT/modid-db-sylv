use std::fs::File;
use std::io::{BufReader, Read, Write};
use std::time::{Duration, SystemTime};
use ferinth::Ferinth;
use ferinth::structures::project::Project;
use ferinth::structures::search::{Facet, Sort};
use ferinth::structures::version::{Version, VersionFile};
use serde::Deserialize;
use sqlx::{PgPool, Postgres, query, query_as};
use time::OffsetDateTime;
use crate::routes;
use crate::routes::v1::mods::Platform;

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
		get_fucking_jars(&pool)
			.await;
	}
}

pub async fn get_fucking_jars(pool: &PgPool) {
	let allowed_loaders = ["quilt", "fabric"];
	let mut facets: Vec<Vec<Facet>> = vec![];
	
	for loader in allowed_loaders {
		facets.push(vec![Facet::Categories(String::from(loader))]);
	}
	let facets: Vec<&[Facet]> = facets.iter().map(|term| term.as_slice()).collect();
	
	let fer = Ferinth::new("SylvKT@github.com/modid-db-sylv", None, Some("mailto:contact@sylv.gay") /* just in case they didn't get the memo */, None)
		.expect("Failed to initialize Ferinth instance");
	
	// Request newest projects
	let res = fer.search("", &Sort::Newest, facets.as_slice())
		.await
		.expect("Failed to search for projects");
	
	// put this shit here because fuck it, we ball
	async fn download_file_from_ver(ver: &Version) -> File {
		// i like boys
		let mut primary: Option<&VersionFile> = None;
		for ver_file in &ver.files {
			if ver_file.primary {
				primary = Some(ver_file);
				break
			}
		}
		
		let ver_file = primary.unwrap_or(ver.files.first().unwrap()); // use first because i'm lazy
		let res = reqwest::get(ver_file.url.clone() /* i'm so sorry, ferris */)
			.await
			.expect(&*format!("Failed to download {} for version {} ({})", ver_file.filename, ver.name, ver.version_number));
		let mut file = File::create(&ver_file.filename)
			.expect(&*format!("Failed to create file {} for version {} ({})", ver_file.filename, ver.name, ver.version_number));
		let bytes = res.bytes()
			.await
			.expect("Failed to get bytes from response");
		let buf: Vec<u8> = bytes.to_vec();
		file.write(buf.as_slice()).expect("Failed to write to file");
		file
	}
	
	// Request each project's latest .jar
	let mut projects: Vec<(Project, String)> = vec![];
	for hit in res.hits {
		let project = fer.get_project(&*hit.project_id)
			.await
			.expect("Failed to retrieve project");
		println!("Downloading first two jar's for {} ({}/{})...", project.title, project.slug, project.id);
		
		let mut hit_version = false; // if a project has passed the filter and have been catalogued
		let mut hit_version_file: Option<File> = None;
		for ver_id in project.versions {
			let ver = fer.get_version(&ver_id)
				.await
				.expect("Failed to get jar");
			if !ver.loaders.iter().any(|x| allowed_loaders.contains(&x.as_str())) {
				break // ;-; haha just like me fr r/im14andthisisdeep r/ihavereddit
			}
			
			hit_version_file = Some(download_file_from_ver(&ver).await);
			hit_version = true;
			// we downloaded the file! yay!!
			println!("Successfully downloaded .jar's for {} ({}/{})!", project.title, project.slug, project.id);
			break
		}
		
		if hit_version { // notify the user
			println!("Successfully downloaded .jar for {} ({}/{})!", project.title, project.slug, project.id);
		}

		// Retrieve the mod ID from the fabric.mod.json or quilt.mod.json
		let id = {
			let mut id_ret = String::new();
			// get id from latest version
			let file = &hit_version_file.unwrap();
			let reader = BufReader::new(file);
			let archive = zip::ZipArchive::new(reader).expect("Failed to open archive");
			for filename in archive.file_names() {
				// open it a second time because we immutably borrowed it
				let reader = BufReader::new(file);
				let mut archive = zip::ZipArchive::new(reader).expect("Failed to open archive");

				let mut file = archive.by_name(filename)
					.expect("Failed to open embedded file in zip");
				let mut string = String::new();
				file.read_to_string(&mut string)
					.expect("Failed to read file");
				match filename {
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
					_ => {
						println!("Mod has no fabric.mod.json or quilt.mod.json");
						println!("This should not happen unless there is a fork of Fabric or Quilt!");
					}
				}
			}
			id_ret
		};

		let project = fer.get_project(&*hit.project_id)
			.await
			.expect("Failed to retrieve project");
		projects.push((project, id));
	}

	for (project, id) in projects {
		// query database with project id
		let mod_opt = query_as!(
			routes::v1::mods::Mod,
			r#"SELECT id, name, description, thumbnail, project_id, platform as "platform: _" FROM mods WHERE project_id = $1"#,
			project.id.to_string()
		)
			.fetch_optional(&*pool)
			.await
			.expect("Failed to query the database");
		if let Some(r#mod) = mod_opt {
			if r#mod.id != id {
				query!(
					"UPDATE mods SET id = $1 WHERE project_id = $2",
					id,
					project.id.to_string()
				)
					.execute(&*pool)
					.await
					.expect("Failed to set mod ID");
			}
		} else {
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
				.execute(&*pool)
				.await
				.expect("Failed to insert new mod to database");
		}
	}
}
