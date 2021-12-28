use celery::prelude::*;

#[celery::task]
fn add(x: i32, y: i32) -> TaskResult<i32> {
    Ok(x + y)
}

#[tokio::test]
async fn test_add() {
    let my_app = celery::app!(
        broker = AMQPBroker { "amqp://localhost:5672//" },
        tasks = [add],
        task_routes = [
            "*" => "celery",
        ],
    ).await.unwrap();

    my_app.send_task(add::new(1, 2)).await.unwrap();
}