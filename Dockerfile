FROM rust:1.57
WORKDIR /work
RUN cargo install sqlx-cli --no-default-features --features postgres,rustls
RUN wget https://github.com/palfrey/wait-for-db/releases/download/v1.2.0/wait-for-db-linux-x86 && chmod +x wait-for-db-linux-x86 && mv wait-for-db-linux-x86 /usr/local/bin/wait-for-db
COPY Cargo.lock Cargo.toml /work/
COPY src/ /work/src/
RUN cargo build
COPY migrations/ /work/migrations/
COPY sqlx-data.json /work/
CMD ./target/debug/cavalcade