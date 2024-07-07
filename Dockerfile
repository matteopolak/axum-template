
FROM rust:alpine AS chef

RUN apk add --no-cache musl-dev
ENV SQLX_OFFLINE=true

RUN cargo install cargo-chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin axum-template

FROM alpine AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/axum-template /usr/local/bin
ENTRYPOINT ["/usr/local/bin/axum-template"]

