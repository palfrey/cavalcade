CREATE TABLE queue (
    id SERIAL PRIMARY KEY,
    _name VARCHAR (256) NOT NULL,
    passive BOOLEAN NOT NULL,
    durable BOOLEAN NOT NULL,
    _exclusive BOOLEAN NOT NULL,
    auto_delete BOOLEAN NOT NULL,
    _nowait BOOLEAN NOT NULL,
    arguments JSONB NOT NULL,
    UNIQUE(_name)
);

CREATE TABLE message (
    id SERIAL PRIMARY KEY,
    arguments JSONB NOT NULL,
    body bytea NOT NULL,
    queue_id INTEGER NOT NULL,
    recieved_at TIMESTAMP,
    consumed_at TIMESTAMP NULL,
    consumed_by UUID NULL,
    FOREIGN KEY (queue_id) REFERENCES queue (id)
);

CREATE TABLE exchange (
    id SERIAL PRIMARY KEY,
    _name VARCHAR (256) NOT NULL,
    _type VARCHAR(256) NOT NULL,
    passive BOOLEAN NOT NULL,
    durable BOOLEAN NOT NULL,
    auto_delete BOOLEAN NOT NULL,
    _nowait BOOLEAN NOT NULL,
    arguments JSONB NOT NULL,
    UNIQUE(_name)
);

CREATE TABLE bind (
    id SERIAL PRIMARY KEY,
    queue_id INTEGER NOT NULL,
    exchange_id INTEGER NOT NULL,
    routing_key VARCHAR (256) NOT NULL,
    _nowait BOOLEAN NOT NULL,
    arguments JSONB NOT NULL,
    FOREIGN KEY (queue_id) REFERENCES queue (id),
    FOREIGN KEY (exchange_id) REFERENCES exchange (id)
);
