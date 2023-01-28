FROM rust:1.66.1-alpine3.17
RUN apk add --no-cache musl-dev
RUN cargo install cargo-watch
WORKDIR /program
CMD cargo watch --ignore target-docker --exec 'run --target-dir=target-docker'
