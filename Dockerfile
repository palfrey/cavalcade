FROM rust:1.66
WORKDIR /work
RUN cargo install sqlx-cli --version ^0.5 --no-default-features --features postgres,rustls
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
COPY sqlx-data.json log4rs.yml /work/
RUN cargo build
COPY migrations/ /work/migrations/
CMD ./target/debug/cavalcade