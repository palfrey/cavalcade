CREATE TABLE queue (
    id UUID PRIMARY KEY,
    _name VARCHAR (256) NOT NULL,
    passive BOOLEAN NOT NULL,
    durable BOOLEAN NOT NULL,
    _exclusive BOOLEAN NOT NULL,
    auto_delete BOOLEAN NOT NULL,
    _nowait BOOLEAN NOT NULL,
    arguments JSONB NOT NULL,
    UNIQUE(_name)
);

CREATE TABLE exchange (
    id UUID PRIMARY KEY,
    _name VARCHAR (256) NOT NULL,
    _type VARCHAR(256) NOT NULL,
    passive BOOLEAN NOT NULL,
    durable BOOLEAN NOT NULL,
    auto_delete BOOLEAN NOT NULL,
    _nowait BOOLEAN NOT NULL,
    arguments JSONB NOT NULL,
    UNIQUE(_name)
);

CREATE TABLE message (
    id UUID PRIMARY KEY,
    arguments JSONB NOT NULL,
    body bytea NOT NULL,
    queue_id UUID NOT NULL,
    recieved_at TIMESTAMP,
    consumed_at TIMESTAMP NULL,
    consumed_by UUID NULL,
    delivery_mode INT4 NULL,
    _priority INT4 NULL,
    correlation_id VARCHAR(256),
    reply_to VARCHAR(256),
    exchange_id UUID NULL,
    routing_key VARCHAR (256) NULL,
    content_type VARCHAR(256) NULL,
    content_encoding VARCHAR(256) NULL,
    FOREIGN KEY (exchange_id) REFERENCES exchange (id),
    FOREIGN KEY (queue_id) REFERENCES queue (id)
);

CREATE TABLE bind (
    id UUID PRIMARY KEY,
    queue_id UUID NOT NULL,
    exchange_id UUID NOT NULL,
    routing_key VARCHAR (256) NOT NULL,
    _nowait BOOLEAN NOT NULL,
    arguments JSONB NOT NULL,
    FOREIGN KEY (queue_id) REFERENCES queue (id),
    FOREIGN KEY (exchange_id) REFERENCES exchange (id)
);
