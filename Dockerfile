# see https://whitfin.io/speeding-up-rust-docker-builds/

FROM rust:1.41 as build

RUN USER=root cargo new --bin twitch_project 
WORKDIR /twitch_project

RUN cargo install diesel_cli --no-default-features --features sqlite

# Hacky: build deps first with empty project to improve caching
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

RUN cargo build --release
RUN rm src/*.rs

COPY ./src ./src
COPY ./.env ./.env
COPY ./config.toml ./config.toml
COPY ./diesel.toml ./diesel.toml
COPY ./migrations/ ./migrations/

RUN diesel setup
RUN rm ./target/release/deps/twitch_chat_parser*
RUN cargo build --release

FROM fedora:31

COPY --from=build /twitch_project/target/release/twitch_chat_parser .
COPY --from=build /twitch_project/.env .
COPY --from=build /twitch_project/config.toml .
COPY --from=build /twitch_project/db.sqlite .
#RUN apt-get update && apt-get install -y \
#	libsqlite3-0 \
#	libssl1.1 \
#	openssl

CMD ["./twitch_chat_parser"]
