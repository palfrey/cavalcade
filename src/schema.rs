table! {
    bind (id) {
        id -> Int4,
        queue_id -> Nullable<Int4>,
        exchange_id -> Nullable<Int4>,
        routing_key -> Nullable<Varchar>,
        #[sql_name = "_nowait"]
        nowait -> Nullable<Bool>,
        arguments -> Nullable<Jsonb>,
    }
}

table! {
    exchange (id) {
        id -> Int4,
        #[sql_name = "_name"]
        name -> Nullable<Varchar>,
        passive -> Nullable<Bool>,
        durable -> Nullable<Bool>,
        auto_delete -> Nullable<Bool>,
        #[sql_name = "_nowait"]
        nowait -> Nullable<Bool>,
        arguments -> Nullable<Jsonb>,
    }
}

table! {
    message (id) {
        id -> Int4,
        arguments -> Nullable<Jsonb>,
        body -> Nullable<Bytea>,
        queue_id -> Nullable<Int4>,
        recieved_at -> Nullable<Timestamp>,
        consumed_at -> Nullable<Timestamp>,
        consumed_by -> Nullable<Uuid>,
    }
}

table! {
    queue (id) {
        id -> Int4,
        #[sql_name = "_name"]
        name -> Nullable<Varchar>,
        passive -> Nullable<Bool>,
        durable -> Nullable<Bool>,
        #[sql_name = "_exclusive"]
        exclusive -> Nullable<Bool>,
        auto_delete -> Nullable<Bool>,
        #[sql_name = "_nowait"]
        nowait -> Nullable<Bool>,
        arguments -> Nullable<Jsonb>,
    }
}

joinable!(bind -> exchange (exchange_id));
joinable!(bind -> queue (queue_id));
joinable!(message -> queue (queue_id));

allow_tables_to_appear_in_same_query!(bind, exchange, message, queue);
