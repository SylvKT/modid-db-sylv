extern crate alloc;

mod test;
mod task;
mod routes;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::thread;
use std::time::Duration;
use actix_web::{App, get, HttpResponse, HttpServer, web};
use ferinth::Ferinth;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use sqlx::postgres::PgPoolOptions;
use crate::routes::{ApiError, v1};
use crate::task::retrieve_jar::{get_fucking_jars, jar_loop};

#[actix_web::main]
async fn main() {
	// Connect to database
	let pool = PgPoolOptions::new()
		.min_connections(0)
		.max_connections(16)
		.max_lifetime(Duration::from_secs(60))
		.connect(env!("DATABASE_URL"))
		.await
		.expect("Failed to connect to Postgres database.");
	let pool_ref = pool.clone();
	
	// create Ferinth instance
	let fer = Ferinth::new("SylvKT@github.com/modid-db-sylv", None, Some("mailto:contact@sylv.gay") /* just in case they didn't get the memo */, None).expect("Failed to create Ferinth instance");

	// Spawn other runtimes
	let runtime = tokio::runtime::Builder::new_multi_thread()
		.enable_time()
		.enable_io()
		.worker_threads(1)
		.thread_name("jar-scan")
		.build()
		.expect("Failed to create tokio runtime \"jar-scan\"");
	
	let handle = runtime.spawn_blocking(|| {
		println!("Began Jar Retrieval Loop");
		jar_loop(pool)
	});
	
	// Start actix server
	let server = HttpServer::new(move || {
		App::new()
			.app_data(web::Data::new(pool_ref.clone()))
			.app_data(web::Data::new(fer.clone()))
			.service(default)
			.configure(v1::config)
	})
		.bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3000))
		.expect("Failed to bind to address")
		.run()
		.await;
	
	handle
		.await
		.expect("Blocking jar retrieval task panicked")
		.await;
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
