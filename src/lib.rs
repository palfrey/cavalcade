use amq_protocol::{
    frame::{AMQPFrame, ProtocolVersion, WriteContext},
    protocol::{
        connection::{AMQPMethod as ConnMethods, Start},
        AMQPClass,
    },
    types::{FieldTable, LongString},
};
use anyhow::{bail, Result};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

pub async fn server() -> Result<()> {
    // Bind the listener to the address
    let listener = TcpListener::bind("127.0.0.1:5672").await?;
    println!("Listening");

    loop {
        // The second item contains the IP and port of the new connection.
        let (socket, _) = listener.accept().await?;
        process(socket).await?;
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
                Ok((rest, frame)) => {
                    let old_buffer_len = self.buffer.len();
                    self.buffer = rest.to_vec();
                    self.cursor -= old_buffer_len - self.buffer.len();
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
                self.cursor += n;
            }
        }
    }

    fn parse_frame(&self) -> Result<(&[u8], AMQPFrame)> {
        return amq_protocol::frame::parse_frame(&self.buffer[..]).map_err(|e| e.into());
    }

    async fn write_frame(&mut self, frame: AMQPFrame) -> Result<()> {
        let mut write_buffer = vec![];
        write_buffer = amq_protocol::frame::gen_frame(&frame)(WriteContext {
            write: write_buffer,
            position: 0,
        })
        .unwrap()
        .into_inner()
        .0;
        self.stream.write(&write_buffer).await?;
        Ok(())
    }

    async fn close(&mut self) -> Result<()> {
        self.stream.shutdown().await?;
        Ok(())
    }
}

async fn process(socket: TcpStream) -> Result<()> {
    println!("Got socket {:?}", socket);

    let mut connection = Connection::new(socket);

    while let Some(frame) = connection.read_frame().await.unwrap() {
        println!("Frame: {:?}", frame);
        match frame {
            AMQPFrame::ProtocolHeader(version) => {
                if version == ProtocolVersion::amqp_0_9_1() {
                    connection
                        .write_frame(AMQPFrame::Method(
                            0,
                            AMQPClass::Connection(ConnMethods::Start(Start {
                                version_major: 0,
                                version_minor: 9,
                                server_properties: FieldTable::default(),
                                mechanisms: LongString::default(),
                                locales: LongString::default(),
                            })),
                        ))
                        .await?;
                } else {
                    connection
                        .write_frame(AMQPFrame::ProtocolHeader(ProtocolVersion::amqp_0_9_1()))
                        .await?;
                    connection.close().await?
                }
            }
            AMQPFrame::Method(_, _) => todo!(),
            AMQPFrame::Header(_, _, _) => todo!(),
            AMQPFrame::Body(_, _) => todo!(),
            AMQPFrame::Heartbeat(_) => todo!(),
        }
    }
    println!("end process");
    Ok(())
}
