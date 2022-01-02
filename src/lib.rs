#[macro_use]
extern crate diesel;

mod models;
mod schema;

use crate::{models::NewQueue, models::Queue, schema::queue};
use amq_protocol::{
    frame::{AMQPFrame, ProtocolVersion, WriteContext},
    protocol::{
        basic::{AMQPMethod as BasicMethods, CancelOk, ConsumeOk, QosOk},
        channel::{AMQPMethod as ChanMethods, CloseOk as ChanCloseOk, OpenOk as ChanOpenOk},
        connection::{
            AMQPMethod as ConnMethods, CloseOk as ConnCloseOk, OpenOk as ConnOpenOk, Start, Tune,
        },
        exchange::{AMQPMethod as ExchangeMethods, DeclareOk as ExchangeDeclareOk},
        queue::{
            AMQPMethod as QueueMethods, BindOk as QueueBindOk, Declare as QueueDeclare,
            DeclareOk as QueueDeclareOk,
        },
        AMQPClass,
    },
    types::{FieldTable, LongString, ShortString},
};
use anyhow::{bail, Result};
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, PooledConnection},
    PgConnection,
};
use dotenv::dotenv;
use lazy_static::lazy_static;
use log::{debug, info};
use regex::bytes::Regex;
use std::{borrow::Cow, env};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

type Conn = PooledConnection<ConnectionManager<PgConnection>>;

pub fn get_db_connection() -> ConnectionManager<PgConnection> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    ConnectionManager::<PgConnection>::new(&database_url)
}

pub async fn server() -> Result<()> {
    info!("Booting");
    let manager = get_db_connection();
    let pool = r2d2::Pool::builder().max_size(15).build(manager).unwrap();
    info!("Database connection established");
    let listener = TcpListener::bind("0.0.0.0:5672").await?;
    info!("Listening");

    loop {
        // The second item contains the IP and port of the new connection.
        let (socket, _) = listener.accept().await?;
        tokio::spawn(process(pool.get().unwrap(), socket));
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

            // Ensure the buffer has capacity
            if self.buffer.len() == self.cursor {
                // Grow the buffer
                self.buffer.resize(self.cursor * 2, 0);
            }

            // Read into the buffer, tracking the number
            // of bytes read
            let n = self.stream.read(&mut self.buffer[self.cursor..]).await?;

            debug!("Got {} bytes", n);
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
        debug!("Output Frame: {:?}", frame);
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

fn fieldtable_to_json(table: &FieldTable) -> serde_json::Value {
    serde_json::to_value(table.inner()).unwrap()
}

fn store_queue(conn: &Conn, declare: &QueueDeclare) {
    let queue_name = declare.queue.to_string();
    let existing_queues = queue::table
        .filter(queue::name.eq(&queue_name))
        .select(queue::all_columns)
        .load::<Queue>(conn)
        .unwrap();
    if !existing_queues.is_empty() {
        assert_eq!(existing_queues.len(), 1);
        diesel::update(queue::table.filter(queue::id.eq(existing_queues.first().unwrap().id)))
            .set((
                queue::passive.eq(Some(declare.passive)),
                queue::durable.eq(Some(declare.durable)),
                queue::exclusive.eq(Some(declare.exclusive)),
                queue::auto_delete.eq(Some(declare.auto_delete)),
                queue::nowait.eq(Some(declare.nowait)),
                queue::arguments.eq(Some(fieldtable_to_json(&declare.arguments))),
            ))
            .execute(conn)
            .unwrap();
    } else {
        diesel::insert_into(queue::table)
            .values(&NewQueue {
                name: Some(queue_name),
                passive: Some(declare.passive),
                durable: Some(declare.durable),
                exclusive: Some(declare.exclusive),
                auto_delete: Some(declare.auto_delete),
                nowait: Some(declare.nowait),
                arguments: Some(fieldtable_to_json(&declare.arguments)),
            })
            .execute(conn)
            .unwrap();
    }
}

async fn process(conn: Conn, socket: TcpStream) -> Result<()> {
    debug!("Got socket {:?}", socket);

    let mut connection = Connection::new(socket);

    while let Some(frame) = connection.read_frame().await.unwrap() {
        debug!("Input Frame: {:?}", frame);
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
                        debug!("Login {:?}", login);
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
                        store_queue(&conn, &declare);
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
                        debug!("Declared exchange {}", declare.exchange);
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
                info!("Content: {}", string_content);
            }
            AMQPFrame::Heartbeat(_channel) => {}
        }
    }
    info!("end process");
    Ok(())
}
