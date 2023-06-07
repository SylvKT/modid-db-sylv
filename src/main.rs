mod test;
mod task;
mod routes;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::thread;
use std::time::Duration;
use actix_web::{App, get, HttpResponse, HttpServer, web};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use sqlx::postgres::PgPoolOptions;
use crate::routes::{ApiError, v1};
use crate::task::retrieve_jar::jar_loop;

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

	// Spawn other runtimes
	thread::spawn(|| {
		spawn_runtimes(&pool);
	});
	
	// Start actix server
	let pool_ref = pool.clone();
	let server = HttpServer::new(move || {
		App::new()
			.app_data(web::Data::new(pool_ref.clone()))
			.service(default)
			.configure(v1::config)
	})
		.bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 3000))
		.expect("Failed to bind to address")
		.run()
		.await;
}

fn spawn_runtimes(pool: &Pool<Postgres>) {
	let runtime = tokio::runtime::Builder::new_multi_thread()
		.enable_time()
		.worker_threads(1)
		.thread_name("jar-scan")
		.build()
		.expect("Failed to create tokio runtime \"jar-scan\"");

	runtime.spawn(jar_loop(pool));
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
