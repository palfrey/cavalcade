CREATE TABLE queue (
    id SERIAL PRIMARY KEY,
    _name VARCHAR (256),
    passive BOOLEAN,
    durable BOOLEAN,
    _exclusive BOOLEAN,
    auto_delete BOOLEAN,
    _nowait BOOLEAN,
    arguments JSONB,
    UNIQUE(_name)
);

CREATE TABLE message (
    id SERIAL PRIMARY KEY,
    arguments JSONB,
    body bytea,
    queue_id INTEGER,
    recieved_at TIMESTAMP,
    consumed_at TIMESTAMP NULL,
    consumed_by UUID NULL,
    FOREIGN KEY (queue_id) REFERENCES queue (id)
);

CREATE TABLE exchange (
    id SERIAL PRIMARY KEY,
    _name VARCHAR (256),
    passive BOOLEAN,
    durable BOOLEAN,
    auto_delete BOOLEAN,
    _nowait BOOLEAN,
    arguments JSONB,
    UNIQUE(_name)
);

CREATE TABLE bind (
    id SERIAL PRIMARY KEY,
    queue_id INTEGER,
    exchange_id INTEGER,
    routing_key VARCHAR (256),
    _nowait BOOLEAN,
    arguments JSONB,
    FOREIGN KEY (queue_id) REFERENCES queue (id),
    FOREIGN KEY (exchange_id) REFERENCES exchange (id)
);
