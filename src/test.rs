#![cfg(test)]

// to the left
// take it back now y'all

use std::time::Duration;
use sqlx::postgres::PgPoolOptions;
use crate::task::retrieve_jar::get_fucking_jars;

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
