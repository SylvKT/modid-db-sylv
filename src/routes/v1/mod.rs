pub mod mod_db;

pub fn config(cfg: &mut actix_web::web::ServiceConfig) {
	cfg.service(
		actix_web::web::scope("v1")
			.configure(mod_db::config)
	);
}
