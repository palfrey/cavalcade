use std::borrow::Cow;

use amq_protocol::{
    frame::{AMQPFrame, ProtocolVersion, WriteContext},
    protocol::{
        basic::{AMQPMethod as BasicMethods, CancelOk, ConsumeOk, QosOk},
        channel::{AMQPMethod as ChanMethods, CloseOk as ChanCloseOk, OpenOk as ChanOpenOk},
        connection::{
            AMQPMethod as ConnMethods, CloseOk as ConnCloseOk, OpenOk as ConnOpenOk, Start, Tune,
        },
        exchange::{AMQPMethod as ExchangeMethods, DeclareOk as ExchangeDeclareOk},
        queue::{AMQPMethod as QueueMethods, BindOk as QueueBindOk, DeclareOk as QueueDeclareOk},
        AMQPClass,
    },
    types::{FieldTable, LongString, ShortString},
};
use anyhow::{bail, Result};
use lazy_static::lazy_static;
use regex::bytes::Regex;
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
        tokio::spawn(process(socket));
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
                Err(_err) => {}
            }

            // println!("getting more data at cursor {}", self.cursor);

            // Ensure the buffer has capacity
            if self.buffer.len() == self.cursor {
                // Grow the buffer
                self.buffer.resize(self.cursor * 2, 0);
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
        amq_protocol::frame::parse_frame(&self.buffer[..]).map_err(|e| e.into())
    }

    async fn write_frame(&mut self, frame: AMQPFrame) -> Result<()> {
        println!("Output Frame: {:?}", frame);
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
        println!("Input Frame: {:?}", frame);
        match frame {
            AMQPFrame::ProtocolHeader(version) => {
                if version == ProtocolVersion::amqp_0_9_1() {
                    connection
                        .write_frame(AMQPFrame::Method(
                            0,
                            AMQPClass::Connection(ConnMethods::Start(Start {
                                version_major: 0,
                                version_minor: 1,
                                server_properties: FieldTable::default(),
                                mechanisms: LongString::from("PLAIN"),
                                locales: LongString::from("en_US"),
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
            AMQPFrame::Method(channel, method) => match method {
                AMQPClass::Connection(connmethod) => match connmethod {
                    ConnMethods::StartOk(start_ok) => {
                        if start_ok.mechanism != ShortString::from("PLAIN") {
                            todo!();
                        }
                        lazy_static! {
                            static ref NULL_SPLIT: Regex = Regex::new("\u{0}").unwrap();
                        }
                        let bytes_response = start_ok.response.as_str().as_bytes();
                        let login: Vec<_> = NULL_SPLIT
                            .splitn(bytes_response, 3)
                            .map(String::from_utf8_lossy)
                            .collect();
                        println!("Login {:?}", login);
                        if login.get(1).unwrap_or(&Cow::from("")) != "guest"
                            || login.get(2).unwrap_or(&Cow::from("")) != "guest"
                        {
                            todo!("Support non guest/guest logins");
                        }
                        connection
                            .write_frame(AMQPFrame::Method(
                                0,
                                AMQPClass::Connection(ConnMethods::Tune(Tune {
                                    channel_max: 1000,
                                    frame_max: 1000,
                                    heartbeat: 1000,
                                })),
                            ))
                            .await?;
                    }
                    ConnMethods::Close(_close) => {
                        connection
                            .write_frame(AMQPFrame::Method(
                                0,
                                AMQPClass::Connection(ConnMethods::CloseOk(ConnCloseOk {})),
                            ))
                            .await?;
                    }
                    ConnMethods::TuneOk(_) => {}
                    ConnMethods::Open(open) => {
                        if open.virtual_host == ShortString::from("/") {
                            connection
                                .write_frame(AMQPFrame::Method(
                                    0,
                                    AMQPClass::Connection(ConnMethods::OpenOk(ConnOpenOk {})),
                                ))
                                .await?;
                        }
                    }
                    _ => todo!(),
                },
                AMQPClass::Channel(chanmethod) => match chanmethod {
                    ChanMethods::Open(_open) => {
                        connection
                            .write_frame(AMQPFrame::Method(
                                channel,
                                AMQPClass::Channel(ChanMethods::OpenOk(ChanOpenOk {})),
                            ))
                            .await?;
                    }
                    ChanMethods::Close(_close) => {
                        connection
                            .write_frame(AMQPFrame::Method(
                                channel,
                                AMQPClass::Channel(ChanMethods::CloseOk(ChanCloseOk {})),
                            ))
                            .await?;
                        // connection.close().await?
                    }
                    _ => todo!(),
                },
                AMQPClass::Queue(queuemethod) => match queuemethod {
                    QueueMethods::Declare(declare) => {
                        println!("Declared queue {}", declare.queue);
                        connection
                            .write_frame(AMQPFrame::Method(
                                channel,
                                AMQPClass::Queue(QueueMethods::DeclareOk(QueueDeclareOk {
                                    queue: declare.queue,
                                    message_count: 0,
                                    consumer_count: 0,
                                })),
                            ))
                            .await?;
                    }
                    QueueMethods::Bind(_bind) => {
                        connection
                            .write_frame(AMQPFrame::Method(
                                channel,
                                AMQPClass::Queue(QueueMethods::BindOk(QueueBindOk {})),
                            ))
                            .await?;
                    }
                    _ => todo!(),
                },
                AMQPClass::Basic(basicmethod) => match basicmethod {
                    BasicMethods::Publish(_publish) => {}
                    BasicMethods::Qos(_qos) => {
                        connection
                            .write_frame(AMQPFrame::Method(
                                channel,
                                AMQPClass::Basic(BasicMethods::QosOk(QosOk {})),
                            ))
                            .await?;
                    }
                    BasicMethods::Consume(consume) => {
                        connection
                            .write_frame(AMQPFrame::Method(
                                channel,
                                AMQPClass::Basic(BasicMethods::ConsumeOk(ConsumeOk {
                                    consumer_tag: consume.consumer_tag,
                                })),
                            ))
                            .await?;
                    }
                    BasicMethods::Cancel(cancel) => {
                        connection
                            .write_frame(AMQPFrame::Method(
                                channel,
                                AMQPClass::Basic(BasicMethods::CancelOk(CancelOk {
                                    consumer_tag: cancel.consumer_tag,
                                })),
                            ))
                            .await?;
                    }
                    _ => todo!(),
                },
                AMQPClass::Exchange(exchangemethod) => match exchangemethod {
                    ExchangeMethods::Bind(_bind) => {}
                    ExchangeMethods::Declare(declare) => {
                        println!("Declared exchange {}", declare.exchange);
                        connection
                            .write_frame(AMQPFrame::Method(
                                channel,
                                AMQPClass::Exchange(ExchangeMethods::DeclareOk(
                                    ExchangeDeclareOk {},
                                )),
                            ))
                            .await?;
                    }
                    _ => todo!(),
                },
                _ => todo!(),
            },
            AMQPFrame::Header(_channel, _class_id, _content) => {}
            AMQPFrame::Body(_channel, content) => {
                let string_content = String::from_utf8_lossy(&content);
                println!("Content: {}", string_content);
            }
            AMQPFrame::Heartbeat(_) => todo!(),
        }
    }
    println!("end process");
    Ok(())
}
