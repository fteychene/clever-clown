FROM rust:1.81 as builder
WORKDIR /usr/src/rokku
COPY . .
RUN cargo build --release
FROM ubuntu:noble
RUN apt-get update && apt-get install -y libssl-dev ca-certificates git && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/src/rokku/target/release/rokku /usr/local/bin/rokku
CMD ["/usr/local/bin/rokku"]