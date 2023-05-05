use std::env;
use std::fs::write;
use std::process::exit;

#[tokio::main]
async fn main() {
	let github_output_path = env::var("GITHUB_OUTPUT").expect("run this in a github action, you goober");
	
	let args: Vec<String> = env::args().collect();
	let error = &args[1];
	
	let client = reqwest::Client::builder()
		.user_agent("SylvKT@github.com/modid-db-sylv (sylv.gay)") // gotta make sure they know it's me, else they'll block the connection if we spam too much.
		.build().unwrap();
	
	// Request newest projects
	let res = client.get("https://api.modrinth.com/v2/search")
		.body(r#"\
		{
			"query": "",
			"index": "newest",
			"offset": 0,
			"limit": 1,
			"filters": "categories=quilt AND categories=fabric"
		}\
		"#) // hello modrinth employees! sorry for spamming, just doing some routine GDPR breaches :3
		.send()
		.await
		.expect("Failed to request newest projects from Modrinth");
	let text = res
		.text()
		.await
		.expect("Failed to parse text from request");
	
	println!("body: {}", text);
	
	// Request each project's latest .jar
	
	// Unzip the .jar and retrieve the mod ID from the fabric.mod.json or quilt.mod.json
	
	if !error.is_empty() {
		eprintln!("Error: {error}");
		write(github_output_path, format!("error={error}")).unwrap();
		exit(1);
	}
}