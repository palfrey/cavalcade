use simplelog::{Config, SimpleLogger};

#[tokio::main]
async fn main() {
    SimpleLogger::init(simplelog::LevelFilter::Debug, Config::default()).unwrap();
    cavalcade::server().await.unwrap()
}
