#[tokio::main]
async fn main() {
    match log4rs::init_file("log4rs.yml", Default::default()) {
        Ok(_) => {}
        Err(err) => {
            println!("Error while trying to load log4rs.yml: {}", err);
        }
    }
    cavalcade::server().await.unwrap()
}
