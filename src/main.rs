use simplelog::{Config, SimpleLogger};

#[tokio::main]
async fn main() {
    SimpleLogger::init(simplelog::LevelFilter::Info, Config::default()).unwrap();
    cavalcade::server().await.unwrap()
}
