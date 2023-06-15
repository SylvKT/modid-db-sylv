use actix_web::web::ServiceConfig;

pub mod mods;

pub fn config(cfg: &mut ServiceConfig) {
	cfg.service(
		actix_web::web::scope("/v0")
			.configure(mods::config)
	);
}
