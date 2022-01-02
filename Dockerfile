FROM rust:1.57
WORKDIR /work
RUN cargo install diesel_cli --no-default-features --features postgres
RUN wget https://github.com/palfrey/wait-for-db/releases/download/v1.2.0/wait-for-db-linux-x86 && chmod +x wait-for-db-linux-x86 && mv wait-for-db-linux-x86 /usr/local/bin/wait-for-db
COPY Cargo.lock Cargo.toml /work/
COPY migrations/* /work/migrations/
COPY src/* /work/src/
RUN cargo build
CMD ./target/debug/cavalcade