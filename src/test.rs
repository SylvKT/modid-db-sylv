#![cfg(test)]

// to the left
// take it back now y'all

use std::os::unix::ffi::OsStringExt;
use std::path::Path;
use std::time::Duration;
use sqlx::postgres::PgPoolOptions;
use tokio::fs::read_dir;
use crate::task::retrieve_jar::{get_fucking_jars, get_id_from_jar};

#[actix_web::test]
pub async fn jar_test() {
	// Connect to database
	let pool = PgPoolOptions::new()
		.min_connections(0)
		.max_connections(16)
		.max_lifetime(Duration::from_secs(60))
		.connect(env!("DATABASE_URL"))
		.await
		.expect("Failed to connect to Postgres database.");
	
	println!("Checking latest .jar's...");
	get_fucking_jars(&pool).await.expect("Failed to retrieve jars");
}

#[actix_web::test]
pub async fn erroneous_jar_test() {
	// Connect to database
	let pool = PgPoolOptions::new()
		.min_connections(0)
		.max_connections(16)
		.max_lifetime(Duration::from_secs(60))
		.connect(env!("DATABASE_URL"))
		.await
		.expect("Failed to connect to Postgres database.");
	
	println!("Opening erroneous .jar's...");
	let mut dir = read_dir(Path::new(".")).await.expect("Failed to read directory");
	loop {
		let entry_opt = dir.next_entry().await.expect("Failed to find entry");
		if let Some(entry) = entry_opt {
			if entry.file_type().await.expect("Failed to get file type").is_file() && String::from_utf8(entry.file_name().into_vec()).expect("Failed to convert to string").ends_with(".jar") {
				let id = get_id_from_jar(entry.path()).await.expect("Failed to get ID");
				println!("{}", id);
			}
		}
	}
}
