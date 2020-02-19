# see https://whitfin.io/speeding-up-rust-docker-builds/

FROM rust:latest as diesel-cli-build
WORKDIR /diesel-cli-install
RUN cargo install --root /diesel-cli-install diesel_cli --no-default-features --features sqlite
RUN ls /
RUN ls /diesel-cli-install

FROM rust:1.41 as build

RUN USER=root cargo new --bin twitch_project 
WORKDIR /twitch_project


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

COPY --from=diesel-cli-build /diesel-cli-install/bin/diesel /bin/diesel
RUN /bin/diesel setup

RUN rm ./target/release/deps/twitch_chat_parser*
RUN cargo build --release

FROM fedora:31

COPY --from=build /twitch_project/target/release/twitch_chat_parser .
COPY --from=build /twitch_project/.env .
COPY --from=build /twitch_project/config.toml .
COPY --from=build /twitch_project/db.sqlite .

CMD ["./twitch_chat_parser"]
