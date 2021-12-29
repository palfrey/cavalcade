#[tokio::main]
async fn main() {
    cavalcade::server().await.unwrap()
}
