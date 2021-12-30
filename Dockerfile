FROM rust:1.57
WORKDIR /work
COPY Cargo.lock Cargo.toml /work/
COPY src/* /work/src/
RUN cargo build
CMD ./target/debug/cavalcade