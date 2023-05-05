use crate::main;

#[tokio::test]
async fn test_main() {
	async {
		main() // make the fucking thing work with cargo test
	}.await
}
