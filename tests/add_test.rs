use std::time::Duration;

use celery::prelude::*;
use serial_test::serial;

#[celery::task]
fn add(x: i32, y: i32) -> TaskResult<i32> {
    Ok(x + y)
}

async fn test_add_core(url: &str) {
    let _ = log4rs::init_file("tests/log4rs.yml", Default::default());
    tokio::spawn(cavalcade::server());
    tokio::time::sleep(Duration::from_secs(1)).await;
    let my_app = celery::app!(
        broker = AMQPBroker { url },
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

#[tokio::test]
#[serial]
async fn test_add_with_plain() {
    test_add_core("amqp://127.0.0.1:5672?auth_mechanism=plain").await;
}

#[tokio::test]
#[serial]
async fn test_add_with_amqplain() {
    test_add_core("amqp://127.0.0.1:5672?auth_mechanism=amqplain").await;
}
