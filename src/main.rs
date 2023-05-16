mod test;
mod task;
mod structures;

use crate::task::retrieve_jar::jar_loop;

fn main() {
	let runtime = tokio::runtime::Builder::new_multi_thread()
		.worker_threads(1)
		.thread_name("jar-scan")
		.build()
		.expect("Failed to create tokio runtime \"jar-scan\"");
	
	runtime.spawn(jar_loop());
}
