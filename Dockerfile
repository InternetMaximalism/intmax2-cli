FROM --platform=$BUILDPLATFORM rust:latest AS builder

RUN apt-get update && apt-get install -y \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

RUN rustup toolchain install nightly \
    && rustup default nightly

WORKDIR /usr/src/app

COPY . .

RUN cargo build -r
RUN cargo build --package store-vault-server -r
RUN cargo build --package withdrawal-server -r
RUN cargo build --package block-builder -r

FROM rust:latest

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /usr/src/app/target/release/store-vault-server /app/
COPY --from=builder /usr/src/app/target/release/withdrawal-server /app/
COPY --from=builder /usr/src/app/target/release/block-builder /app/

RUN chmod +x /app/store-vault-server \
    && chmod +x /app/withdrawal-server \
    && chmod +x /app/block-builder

ARG PORT

ENV PORT=${PORT}
EXPOSE ${PORT}

ENTRYPOINT ["/bin/sh", "-c"]