use std::time::Duration;

use celery::prelude::*;
use simplelog::{Config, SimpleLogger};

#[celery::task]
fn add(x: i32, y: i32) -> TaskResult<i32> {
    Ok(x + y)
}

#[tokio::test]
async fn test_add() {
    SimpleLogger::init(simplelog::LevelFilter::Debug, Config::default()).unwrap();
    tokio::spawn(cavalcade::server());
    tokio::time::sleep(Duration::from_secs(1)).await;
    let my_app = celery::app!(
        broker = AMQPBroker { "amqp://127.0.0.1:5672//" },
        tasks = [add],
        task_routes = [
            "*" => "celery",
        ],
    )
    .await
    .unwrap();

    my_app.send_task(add::new(1, 2)).await.unwrap();
    my_app.close().await.unwrap();
}
