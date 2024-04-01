FROM --platform=$BUILDPLATFORM rust:bookworm as vendor
ARG BUILDPLATFORM
ARG TARGETPLATFORM
RUN echo "Running on: $BUILDPLATFORM / Building for $TARGETPLATFORM"
WORKDIR /app

COPY ./Cargo.toml .
#COPY ./Cargo.lock .
COPY ./src src
RUN mkdir .cargo && cargo vendor > .cargo/config.toml

FROM rust:bookworm as builder
WORKDIR /app

COPY --from=vendor /app/.cargo .cargo
COPY --from=vendor /app/vendor vendor
COPY ./Cargo.toml .
#COPY ./Cargo.lock .
COPY ./src src
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
ENV RUST_BACKTRACE=full
COPY --from=builder /app/target/release/livebox-exporter-rs livebox-exporter-rs

EXPOSE 9100
ENTRYPOINT ["/app/livebox-exporter-rs"]
