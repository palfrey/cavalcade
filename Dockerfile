FROM rust:1.57
WORKDIR /work
RUN cargo install diesel_cli --no-default-features --features postgres
COPY Cargo.lock Cargo.toml /work/
COPY src/* /work/src/
RUN cargo build
CMD ./target/debug/cavalcade