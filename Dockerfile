FROM rust:1.81 as builder
WORKDIR /usr/src/cleverclown
COPY . .
RUN cargo build --release

FROM ubuntu:noble
RUN apt-get update && apt-get install -y libssl-dev ca-certificates git && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/cleverclown/target/release/cleverclown /usr/local/bin/cleverclown
CMD ["/usr/local/bin/cleverclown"]