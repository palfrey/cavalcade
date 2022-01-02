use diesel::{Insertable, Queryable};

use super::schema::queue;

#[derive(Insertable)]
#[table_name = "queue"]
pub struct NewQueue {
    pub name: Option<String>,
    pub passive: Option<bool>,
    pub durable: Option<bool>,
    pub exclusive: Option<bool>,
    pub auto_delete: Option<bool>,
    pub nowait: Option<bool>,
    pub arguments: Option<serde_json::Value>,
}

#[derive(Queryable, Debug)]
pub struct Queue {
    pub id: i32,
    pub name: Option<String>,
    pub passive: Option<bool>,
    pub durable: Option<bool>,
    pub exclusive: Option<bool>,
    pub auto_delete: Option<bool>,
    pub nowait: Option<bool>,
    pub arguments: Option<serde_json::Value>,
}
