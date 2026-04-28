FROM rust:1-slim-bookworm AS builder

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
COPY meerkat-server/Cargo.toml meerkat-server/Cargo.toml
RUN mkdir -p meerkat-server/src && echo "fn main() {}" > meerkat-server/src/main.rs
RUN cargo build --release --bin meerkat-server
RUN rm -rf meerkat-server/src

COPY meerkat-server/src meerkat-server/src
COPY meerkat-server/tests meerkat-server/tests
RUN touch meerkat-server/src/main.rs
RUN cargo build --release --bin meerkat-server


FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --uid 1000 meerkat
USER meerkat
WORKDIR /home/meerkat

COPY --from=builder /build/target/release/meerkat-server /usr/local/bin/meerkat-server

EXPOSE 8000

CMD ["meerkat-server"]
