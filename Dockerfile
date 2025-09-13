# inspiration: https://dev.to/rogertorres/first-steps-with-docker-rust-30oi

FROM rust:1.89-bookworm AS build

# create an empty shell project
RUN USER=root cargo new --bin rusteze
WORKDIR /rusteze

# copy manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./Cargo.toml ./Cargo.toml

# cache dependencies
RUN cargo build --release
RUN rm -r ./src

# copy real source
COPY ./src ./src

# build for release
RUN rm ./target/release/rusteze*
RUN find ./src/ -exec touch '{}' ';'
RUN cargo build --release

# executing image
FROM debian:bookworm-slim

COPY --from=build /rusteze/target/release/rusteze .

ENTRYPOINT ["./rusteze"]
