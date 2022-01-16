use std::process::{exit, Command};

use clap::{App, Arg};

#[tokio::main]
async fn main() {
    let matches = App::new("cavalcade")
        .version(clap::crate_version!())
        .arg(
            Arg::new("migrate")
                .long("migrate")
                .help("Run sqlx migrations"),
        )
        .arg(
            Arg::new("sqlx-path")
                .long("sqlx-path")
                .help("Path to sqlx")
                .default_value("sqlx")
                .takes_value(true),
        )
        .get_matches();

    if matches.is_present("migrate") {
        let sqlx_path = matches.value_of("sqlx-path").unwrap();
        if !Command::new(sqlx_path)
            .args(["migrate", "info"])
            .spawn()
            .unwrap_or_else(|_| panic!("Expected sqlx spawn to work for path '{}'", sqlx_path))
            .wait()
            .expect("Migration failure")
            .success()
        {
            exit(-1);
        }

        if !Command::new(sqlx_path)
            .args(["migrate", "run"])
            .spawn()
            .unwrap_or_else(|_| panic!("Expected sqlx spawn to work for path '{}'", sqlx_path))
            .wait()
            .expect("Migration failure")
            .success()
        {
            exit(-1);
        }
        return;
    }

    match log4rs::init_file("log4rs.yml", Default::default()) {
        Ok(_) => {}
        Err(err) => {
            println!("Error while trying to load log4rs.yml: {}", err);
        }
    }
    cavalcade::server().await.unwrap()
}
