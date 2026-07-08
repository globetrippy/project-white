FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY install.sh install.ps1 ./
COPY src ./src

RUN cargo build --release --bin pw-server && \
    cargo build --release --bin pw && \
    cp target/release/pw-server /pw-server && \
    cp target/release/pw /pw-cli

FROM alpine:3.21

RUN apk add --no-cache ca-certificates tzdata && \
    adduser -D -u 1000 pw

COPY --from=builder /pw-server /usr/local/bin/pw-server
COPY --from=builder /pw-cli /usr/local/bin/pw-cli

USER pw

ENV PW_SERVER_ADDR=0.0.0.0:8080

EXPOSE 8080

ENTRYPOINT ["pw-server"]
