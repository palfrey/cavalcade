use std::time::Duration;

use celery::prelude::*;

#[celery::task]
fn add(x: i32, y: i32) -> TaskResult<i32> {
    Ok(x + y)
}

#[tokio::test]
async fn test_add() {
    let handle = tokio::spawn(cavalcade::server());    
    tokio::time::sleep(Duration::from_secs(1)).await;
    let my_app = celery::app!(
        broker = AMQPBroker { "amqp://127.0.0.1:5672//" },
        tasks = [add],
        task_routes = [
            "*" => "celery",
        ],
    ).await.unwrap();

    my_app.send_task(add::new(1, 2)).await.unwrap();
    handle.await.unwrap().unwrap();
}