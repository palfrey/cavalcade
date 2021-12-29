use amq_protocol::frame::AMQPFrame;
use anyhow::{bail, Result};
use tokio::{
    io::AsyncReadExt,
    net::{TcpListener, TcpStream},
};

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
pub struct Connection {
    stream: TcpStream,
    buffer: Vec<u8>,
    cursor: usize,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Connection {
        Connection {
            stream,
            buffer: vec![0; 4096],
            cursor: 0,
        }
    }

    pub async fn read_frame(&mut self) -> Result<Option<AMQPFrame>> {
        loop {
            match self.parse_frame() {
                Ok(frame) => {
                    return Ok(Some(frame));
                }
                Err(err) => {
                    println!("parse error: {}", err);
                }
            }

            println!("getting more data at cursor {}", self.cursor);

            // Ensure the buffer has capacity
            if self.buffer.len() == self.cursor {
                // Grow the buffer
                bail!("resize");
                // println!("resize");
                // self.buffer.resize(self.cursor * 2, 0);
            }

            // Read into the buffer, tracking the number
            // of bytes read
            let n = self.stream.read(&mut self.buffer[self.cursor..]).await?;

            println!("Got {} bytes", n);
            if 0 == n {
                if self.cursor == 0 {
                    return Ok(None);
                } else {
                    bail!("connection reset by peer");
                }
            } else {
                // Update our cursor
                if self.cursor != 0 {
                    bail!("foo");
                }
                self.cursor += n;
            }
        }
    }

    fn parse_frame(&self) -> Result<AMQPFrame> {
        return amq_protocol::frame::parsing::parse_frame(&self.buffer[..])
            .map(|r| r.1)
            .map_err(|e| e.into());
    }
}

async fn process(socket: TcpStream) {
    println!("Got socket {:?}", socket);

    let mut connection = Connection::new(socket);

    while let Some(frame) = connection.read_frame().await.unwrap() {
        println!("Frame: {:?}", frame);
    }
    println!("end process");
}
