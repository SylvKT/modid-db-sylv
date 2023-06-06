mod test;
mod task;
mod routes;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;
use actix_web::{App, HttpServer, web};
use sqlx::postgres::PgPoolOptions;
use crate::routes::v1;
use crate::task::retrieve_jar::jar_loop;

fn main() {
	// Spawn main runtime
	let server_runtime = tokio::runtime::Builder::new_current_thread()
		.enable_time()
		.enable_io()
		.worker_threads(1)
		.thread_name("main")
		.build()
		.expect("Failed to create tokio runtime \"main\"");

	server_runtime.spawn(server_main());

	// Spawn other runtimes
	let runtime = tokio::runtime::Builder::new_multi_thread()
		.enable_time()
		.worker_threads(1)
		.thread_name("jar-scan")
		.build()
		.expect("Failed to create tokio runtime \"jar-scan\"");

	//runtime.spawn(jar_loop());
}

async fn server_main() {
	// Connect to database
	let pool = PgPoolOptions::new()
		.min_connections(0)
		.max_connections(16)
		.max_lifetime(Duration::from_secs(60))
		.connect(env!("DATABASE_URL"))
		.await
		.expect("Failed to connect to Postgres database.");
	
	// Start actix server
	let pool_ref = pool.clone();
	let server = HttpServer::new(move || {
		App::new().service(
			web::scope("/")
				.app_data(web::Data::new(pool_ref.clone()))
				.configure(v1::config)
		)
	})
		.bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 443))
		.expect("Failed to bind to address")
		.run();
}
