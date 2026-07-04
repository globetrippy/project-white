FROM rust:1.85-alpine AS builder

RUN apk add --no-cache musl-dev pkgconfig

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src

RUN cargo build --release --bin pw-server && \
    cp target/release/pw-server /pw-server

FROM alpine:3.21

RUN apk add --no-cache ca-certificates tzdata && \
    adduser -D -u 1000 pw

COPY --from=builder /pw-server /usr/local/bin/pw-server

USER pw

ENV PW_SERVER_ADDR=0.0.0.0:8080

EXPOSE 8080

ENTRYPOINT ["pw-server"]
