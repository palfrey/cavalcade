use tokio::net::{TcpListener, TcpStream};
use anyhow::Result;

pub async fn server() -> Result<()> {
    // Bind the listener to the address
    let listener = TcpListener::bind("127.0.0.1:5672").await?;
    println!("Listening");

    loop {
        // The second item contains the IP and port of the new connection.
        let (socket, _) = listener.accept().await?;
        process(socket).await;
    }
}

async fn process(socket: TcpStream) {
    println!("Got socket {:?}", socket);
}