use clap::App;

#[tokio::main]
async fn main() {
    App::new("cavalcade")
        .version(clap::crate_version!())
        .get_matches();
    match log4rs::init_file("log4rs.yml", Default::default()) {
        Ok(_) => {}
        Err(err) => {
            println!("Error while trying to load log4rs.yml: {}", err);
        }
    }
    cavalcade::server().await.unwrap()
}
