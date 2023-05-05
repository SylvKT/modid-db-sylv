use crate::main;

#[tokio::test]
async fn test_main() {
	main().await // make the fucking thing work with cargo test
}
