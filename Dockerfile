FROM rust:latest as builder

WORKDIR /usr/src

RUN cargo new app

COPY Cargo.toml Cargo.lock /usr/src/app/

WORKDIR /usr/src/app

RUN cargo build --release

COPY src /usr/src/app/src

RUN cargo build --release


FROM rust:slim-buster as runtime

COPY --from=builder /usr/src/app/target/release/ /usr/local/bin/
