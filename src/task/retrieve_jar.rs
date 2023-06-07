use std::fs::File;
use std::io::{BufReader, Write};
use std::time::{Duration, SystemTime};
use ferinth::Ferinth;
use ferinth::structures::project::Project;
use ferinth::structures::search::{Facet, Sort};
use ferinth::structures::version::{Version, VersionFile};
use sqlx::{Pool, Postgres};
use time::OffsetDateTime;

pub async fn jar_loop(pool: &Pool<Postgres>) {
	let mut interval = tokio::time::interval(Duration::from_secs(30 * 60));
	loop {
		if OffsetDateTime::from(SystemTime::now()).minute() % 30 == 0 {
			break;
		}
	}
	
	loop {
		interval.tick().await;
		println!("Checking latest .jar's...");
		get_fucking_jars(pool)
			.await;
	}
}

pub async fn get_fucking_jars(pool: &Pool<Postgres>) {
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
	let mut projects: Vec<(Vec<File>, Project, String)> = vec![];
	for hit in res.hits {
		let project = fer.get_project(&*hit.project_id)
			.await
			.expect("Failed to retrieve project");
		println!("Downloading first two jar's for {} ({}/{})...", project.title, project.slug, project.id);
		
		let mut hit_versions = 0; // how many projects passed the filter and have been catalogued
		let mut hit_version_files = vec![];
		for ver_id in project.versions {
			let ver = fer.get_version(&ver_id)
				.await
				.expect("Failed to get jar");
			if !ver.loaders.iter().any(|x| allowed_loaders.contains(&x.as_str())) {
				break // ;-; haha just like me fr r/im14andthisisdeep r/ihavereddit
			}
			
			hit_version_files.push(download_file_from_ver(&ver).await);
			hit_versions += 1;
			if hit_versions >= 2 { // we can now compare the mod IDs
				println!("Successfully downloaded .jar's for {} ({}/{})!", project.title, project.slug, project.id);
				break
			}
		}
		
		if hit_versions == 1 { // notify the user
			println!("Successfully downloaded .jar for {} ({}/{})!", project.title, project.slug, project.id);
		}

		// Retrieve the mod ID from the fabric.mod.json or quilt.mod.json
		let id = {
			// verify all of the IDs are the same
			let ids = vec![];
			for file in hit_version_files {
				let reader = BufReader::new(file);
				let mut archive = zip::ZipArchive::new(reader).expect("Failed to open archive");
			}
		};

		let project = fer.get_project(&*hit.project_id)
			.await
			.expect("Failed to retrieve project");
		projects.push((hit_version_files, project, id));
	}
}
