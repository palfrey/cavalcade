FROM rust:1.72
WORKDIR /work
RUN cargo install sqlx-cli --version ^0.7 --no-default-features --features postgres,rustls
RUN wget https://github.com/palfrey/wait-for-db/releases/download/v1.2.0/wait-for-db-linux-x86 && chmod +x wait-for-db-linux-x86 && mv wait-for-db-linux-x86 /usr/local/bin/wait-for-db

# Creating a new dummy project
RUN cargo init
# Replacing with the desired dependencies
COPY Cargo.lock Cargo.toml /work/
# Build deps
RUN cargo build
# Deleting the dummy project, leaving an environment ready to `run`
RUN rm src/* -rf

COPY src/ /work/src/
COPY log4rs.yml /work/
COPY .sqlx/ /work/.sqlx
RUN cargo build
COPY migrations/ /work/migrations/
CMD ./target/debug/cavalcade