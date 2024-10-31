# bookworm
FROM --platform=$BUILDPLATFORM rust:bookworm@sha256:33a0ea4769482be860174e1139c457bdcb2a236a988580a28c3a48824cbc17d6 as vendor
ARG BUILDPLATFORM
ARG TARGETPLATFORM
WORKDIR /app
COPY ./Cargo.toml .
COPY ./src src
RUN mkdir .cargo && cargo vendor > .cargo/config.toml

# bookworm
FROM rust:bookworm@sha256:33a0ea4769482be860174e1139c457bdcb2a236a988580a28c3a48824cbc17d6 as builder
WORKDIR /app

COPY --from=vendor /app/.cargo .cargo
COPY --from=vendor /app/vendor vendor
COPY ./Cargo.toml .
COPY ./src src
RUN cargo build --release

# bookworm-slim
FROM debian:bookworm-slim@sha256:36e591f228bb9b99348f584e83f16e012c33ba5cad44ef5981a1d7c0a93eca22
WORKDIR /app
ENV RUST_BACKTRACE=full
COPY --from=builder /app/target/release/livebox-exporter-rs livebox-exporter-rs

EXPOSE 9100
ENTRYPOINT ["/app/livebox-exporter-rs"]
