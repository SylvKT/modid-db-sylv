use std::fs::File;
use std::io::BufReader;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use actix_web::{App, get, HttpResponse, HttpServer, web};
use actix_web::http::StatusCode;
use actix_web_lab::header::StrictTransportSecurity;
use actix_web_lab::middleware::RedirectHttps;
use actix_web::middleware::ErrorHandlers;
use ferinth::Ferinth;
use rustls_pemfile::{certs, pkcs8_private_keys};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPoolOptions;

use crate::routes::{ApiError, v0};
use crate::task::retrieve_jar::jar_loop;
use crate::error::{handle_400, handle_404};

mod test;
mod task;
mod routes;
mod util;
mod error;

static USE_TLS: bool = true;

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
			.wrap(RedirectHttps::with_hsts(StrictTransportSecurity::recommended()).to_port(443))
			.app_data(web::Data::new(pool_ref.clone()))
			.app_data(web::Data::new(fer.clone()))
			.service(default)
			.wrap(ErrorHandlers::new().handler(StatusCode::BAD_REQUEST, handle_400))
			.wrap(ErrorHandlers::new().handler(StatusCode::NOT_FOUND, handle_404))
			.configure(v0::config)
	});
	
	if USE_TLS {
		let certs = load_certs()
			.expect("Failed to load certificates");
		
		server
			.bind_rustls(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 443), certs)
			.expect("Failed to bind to address 0.0.0.0 on port 443")
			.bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 80))
			.expect("Failed to bind to address 0.0.0.0 on port 80")
			.run()
			.await
			.expect("Server panicked");
	} else {
		server
			.bind(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 80))
			.expect("Failed to bind to address 0.0.0.0 on port 80")
			.run()
			.await
			.expect("Server panicked");
	}
	
	handle
		.await
		.expect("Blocking jar retrieval task panicked")
		.await;
}

fn load_certs() -> Result<rustls::ServerConfig, ApiError> {
	let cert_file = &mut BufReader::new(File::open("cert.pem")?);
	let key_file = &mut BufReader::new(File::open("key.pem")?);
	
	let cert_chain = certs(cert_file)?
		.into_iter()
		.map(rustls::Certificate)
		.collect();
	let mut keys: Vec<rustls::PrivateKey> = pkcs8_private_keys(key_file)?
		.into_iter()
		.map(rustls::PrivateKey)
		.collect();
	
	// exit if couldn't parse keys
	if keys.is_empty() {
		return Err(ApiError::Other("Failed to locate PKCS 8 private keys".to_string()))
	}
	
	let config = rustls::ServerConfig::builder()
		.with_safe_defaults()
		.with_no_client_auth();
	Ok(config.with_single_cert(cert_chain, keys.remove(0))?)
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
