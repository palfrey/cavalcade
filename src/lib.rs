use amq_protocol::{
    frame::{AMQPFrame, ProtocolVersion, WriteContext},
    protocol::{
        basic::{AMQPMethod as BasicMethods, CancelOk, ConsumeOk, QosOk},
        channel::{AMQPMethod as ChanMethods, CloseOk as ChanCloseOk, OpenOk as ChanOpenOk},
        connection::{
            AMQPMethod as ConnMethods, CloseOk as ConnCloseOk, OpenOk as ConnOpenOk, Start, Tune,
        },
        exchange::{
            AMQPMethod as ExchangeMethods, Declare as ExchangeDeclare,
            DeclareOk as ExchangeDeclareOk,
        },
        queue::{
            AMQPMethod as QueueMethods, Bind, BindOk as QueueBindOk, Declare as QueueDeclare,
            DeclareOk as QueueDeclareOk,
        },
        AMQPClass,
    },
    types::{FieldTable, LongString, ShortString},
};
use anyhow::{bail, Result};
use chrono::Utc;
use dotenv::dotenv;
use lazy_static::lazy_static;
use log::{debug, info};
use regex::{bytes::Regex as BytesRegex, Regex};
use sqlx::PgPool;
use std::{borrow::Cow, env};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpListener, TcpStream,
    },
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
};

async fn get_db_connection() -> Result<PgPool> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    return PgPool::connect(&database_url).await.map_err(|e| e.into());
}

pub async fn server() -> Result<()> {
    info!("Booting");
    let pool = get_db_connection().await?;
    info!("Database connection established");
    let listener = TcpListener::bind("0.0.0.0:5672").await?;
    info!("Listening");

    loop {
        // The second item contains the IP and port of the new connection.
        let (socket, _) = listener.accept().await?;
        tokio::spawn(process(pool.clone(), socket));
    }
}
pub struct ConnectionReader {
    stream: OwnedReadHalf,
    buffer: Vec<u8>,
    cursor: usize,
}

impl ConnectionReader {
    pub fn new(stream: OwnedReadHalf) -> Self {
        Self {
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
}

pub struct ConnectionWriter {
    stream: OwnedWriteHalf,
    receiver: UnboundedReceiver<AMQPFrame>,
}

impl ConnectionWriter {
    pub fn new(stream: OwnedWriteHalf, receiver: UnboundedReceiver<AMQPFrame>) -> Self {
        Self { stream, receiver }
    }

    async fn run(&mut self) {
        loop {
            let msg = self.receiver.recv().await;
            if let Some(frame) = msg {
                self.write_frame(frame);
            } else {
                break;
            }
        }
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

async fn store_queue(conn: &PgPool, declare: &QueueDeclare) {
    let queue_name = declare.queue.to_string();
    let existing_queues = sqlx::query!("SELECT id FROM queue WHERE _name = $1", &queue_name)
        .fetch_one(conn)
        .await;
    let id = existing_queues.ok().map(|r| r.id);
    if id.is_some() {
        sqlx::query!(
            "
            UPDATE queue SET passive = $1, durable = $2, _exclusive = $3, auto_delete = $4, _nowait = $5, arguments = $6
            WHERE id = $7",
            declare.passive,
            declare.durable,
            declare.exclusive, declare.auto_delete, declare.nowait, fieldtable_to_json(&declare.arguments), id.unwrap()
        )
        .execute(conn)
        .await.unwrap();
    } else {
        sqlx::query!("INSERT INTO queue (_name, passive, durable, _exclusive, auto_delete, _nowait, arguments) VALUES($1, $2, $3, $4, $5, $6, $7)",
        queue_name,
        declare.passive,
        declare.durable,
        declare.exclusive, declare.auto_delete, declare.nowait, fieldtable_to_json(&declare.arguments)).execute(conn).await.unwrap();
    }
}

async fn store_exchange(conn: &PgPool, declare: &ExchangeDeclare) {
    let exchange_name = declare.exchange.to_string();
    let existing_exchanges =
        sqlx::query!("SELECT id FROM exchange WHERE _name = $1", &exchange_name)
            .fetch_one(conn)
            .await;
    let id = existing_exchanges.ok().map(|r| r.id);
    if id.is_some() {
        sqlx::query!(
            "
            UPDATE exchange SET passive = $1, durable = $2, auto_delete = $3, _nowait = $4, arguments = $5
            WHERE id = $6",
            declare.passive,
            declare.durable,
            declare.auto_delete, declare.nowait, fieldtable_to_json(&declare.arguments), id.unwrap()
        )
        .execute(conn)
        .await.unwrap();
    } else {
        sqlx::query!("INSERT INTO exchange (_name, _type, passive, durable, auto_delete, _nowait, arguments) VALUES($1, $2, $3, $4, $5, $6, $7)",
        exchange_name,
        declare.kind.to_string(),
        declare.passive,
        declare.durable,
        declare.auto_delete, declare.nowait, fieldtable_to_json(&declare.arguments)).execute(conn).await.unwrap();
    }
}

async fn store_bind(conn: &PgPool, bind: &Bind) {
    let exchange_name = bind.exchange.to_string();
    let queue_name = bind.queue.to_string();
    let routing_key = bind.routing_key.to_string();
    let existing_binds = sqlx::query!(
        "SELECT id FROM bind WHERE 
            queue_id IN (SELECT id FROM queue where _name = $1) AND
            exchange_id IN (SELECT id from exchange where _name = $2) AND
            routing_key = $3",
        &queue_name,
        &exchange_name,
        &routing_key
    )
    .fetch_one(conn)
    .await;
    let id = existing_binds.ok().map(|r| r.id);
    if id.is_some() {
        sqlx::query!(
            "
            UPDATE bind SET _nowait = $1, arguments = $2
            WHERE id = $3",
            bind.nowait,
            fieldtable_to_json(&bind.arguments),
            id.unwrap()
        )
        .execute(conn)
        .await
        .unwrap();
    } else {
        let queue = sqlx::query!("SELECT id FROM queue WHERE _name = $1", queue_name)
            .fetch_one(conn)
            .await;
        let exchange = sqlx::query!("SELECT id FROM exchange WHERE _name = $1", exchange_name)
            .fetch_one(conn)
            .await;
        sqlx::query!("INSERT INTO bind (queue_id, exchange_id, routing_key, _nowait, arguments) VALUES($1, $2, $3, $4, $5)",
            queue.unwrap().id,
            exchange.unwrap().id,
            routing_key,
            bind.nowait,
            fieldtable_to_json(&bind.arguments)).execute(conn).await.unwrap();
    }
}

#[derive(PartialEq, Debug)]
enum MessageType {
    Nothing,
    Publish,
}

#[derive(Debug)]
struct Message {
    kind: MessageType,
    exchange: Option<String>,
    routing_key: Option<String>,
    body_size: Option<u64>,
    headers: Option<serde_json::Value>,
}

async fn insert_into_named_queue(
    conn: &PgPool,
    queue_name: &String,
    message: &Message,
    content: Vec<u8>,
) {
    let queue = sqlx::query!("SELECT id FROM queue WHERE _name = $1", queue_name)
        .fetch_one(conn)
        .await;
    sqlx::query!(
        "INSERT INTO message (arguments, body, queue_id, recieved_at, consumed_at, consumed_by) VALUES($1, $2, $3, $4, NULL, NULL)",
    message.headers, content, queue.unwrap().id, Utc::now().naive_utc()).execute(conn).await.unwrap();
}

async fn store_message(conn: &PgPool, message: &Message, content: Vec<u8>) {
    debug!("Store message: {:?}", message);
    let exchange_name = message.exchange.as_ref().unwrap();
    let routing_key = message.routing_key.as_ref().unwrap();
    if exchange_name == "" {
        insert_into_named_queue(conn, routing_key, message, content).await;
        return;
    }
    let exchange = sqlx::query!(
        "SELECT id, _type FROM exchange WHERE _name = $1",
        &exchange_name
    )
    .fetch_one(conn)
    .await
    .unwrap();
    if exchange._type == "topic" {
        let binds = sqlx::query!(
            "SELECT id, queue_id, routing_key FROM bind WHERE exchange_id = $1",
            &exchange.id
        )
        .fetch_all(conn)
        .await
        .unwrap();
        for bind in binds {
            // * (star) can substitute for exactly one word.
            // # (hash) can substitute for zero or more words.
            let pattern = Regex::new(&str::replace(
                &str::replace(&bind.routing_key, "*", r"[^\.]+"),
                "#",
                r"[^\.]*",
            ))
            .unwrap();
            if pattern.is_match(routing_key) {
                sqlx::query!(
                    "INSERT INTO message (arguments, body, queue_id, recieved_at, consumed_at, consumed_by) VALUES($1, $2, $3, $4, NULL, NULL)",
                message.headers, content, bind.queue_id, Utc::now().naive_utc()).execute(conn).await.unwrap();
            }
        }
    } else {
        panic!("{:?}", exchange);
    }
}

async fn process(conn: PgPool, socket: TcpStream) -> Result<()> {
    debug!("Got socket {:?}", socket);

    let (read_half, write_half) = socket.into_split();

    let mut connection = ConnectionReader::new(read_half);
    let (sender, receiver) = mpsc::unbounded_channel::<AMQPFrame>();
    let writer = ConnectionWriter::new(write_half, receiver);

    tokio::spawn(writer.run());

    let mut current_message = Message {
        kind: MessageType::Nothing,
        exchange: None,
        routing_key: None,
        body_size: None,
        headers: None,
    };

    while let Some(frame) = connection.read_frame().await.unwrap() {
        debug!("Input Frame: {:?}", frame);
        match frame {
            AMQPFrame::ProtocolHeader(version) => {
                if version == ProtocolVersion::amqp_0_9_1() {
                    sender.send(AMQPFrame::Method(
                        0,
                        AMQPClass::Connection(ConnMethods::Start(Start {
                            version_major: 0,
                            version_minor: 1,
                            server_properties: FieldTable::default(),
                            mechanisms: LongString::from("PLAIN"),
                            locales: LongString::from("en_US"),
                        })),
                    ))?;
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
                            static ref NULL_SPLIT: BytesRegex = BytesRegex::new("\u{0}").unwrap();
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
                    }
                    _ => todo!(),
                },
                AMQPClass::Queue(queuemethod) => match queuemethod {
                    QueueMethods::Declare(declare) => {
                        store_queue(&conn, &declare).await;
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
                    QueueMethods::Bind(bind) => {
                        store_bind(&conn, &bind).await;
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
                    BasicMethods::Publish(publish) => {
                        assert_eq!(current_message.kind, MessageType::Nothing);
                        current_message.kind = MessageType::Publish;
                        current_message.exchange = Some(publish.exchange.to_string());
                        current_message.routing_key = Some(publish.routing_key.to_string());
                    }
                    BasicMethods::Qos(_qos) => {
                        connection
                            .write_frame(AMQPFrame::Method(
                                channel,
                                AMQPClass::Basic(BasicMethods::QosOk(QosOk {})),
                            ))
                            .await?;
                    }
                    BasicMethods::Consume(consume) => {
                        tokio::spawn(delivery(
                            conn.clone(),
                            connection.clone(),
                            channel,
                            consume.queue.to_string(),
                        ));
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
                        store_exchange(&conn, &declare).await;
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
            AMQPFrame::Header(_channel, _class_id, content) => {
                assert_ne!(current_message.kind, MessageType::Nothing);
                let headers = content.properties.headers().as_ref().unwrap();
                current_message.headers = Some(fieldtable_to_json(&headers));
                current_message.body_size = Some(content.body_size);
            }
            AMQPFrame::Body(_channel, content) => {
                let string_content = String::from_utf8_lossy(&content);
                info!("Content: {}", string_content);
                assert_eq!(current_message.kind, MessageType::Publish);
                assert_eq!(current_message.body_size.unwrap(), content.len() as u64);
                store_message(&conn, &current_message, content).await;
                current_message.kind = MessageType::Nothing;
            }
            AMQPFrame::Heartbeat(_channel) => {}
        }
    }
    info!("end process");
    Ok(())
}

async fn delivery(
    conn: PgPool,
    sender: UnboundedSender<AMQPFrame>,
    channel: u16,
    to_string: String,
) {
    todo!()
}
