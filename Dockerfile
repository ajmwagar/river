# Build
FROM clux/muslrust:latest as cargo-build 

WORKDIR /usr/src/
RUN USER=root cargo new --bin art
WORKDIR /usr/src/art
COPY ./Cargo.toml ./Cargo.toml 
COPY ./Cargo.lock ./Cargo.lock 
RUN cargo build --release
RUN rm -f target/x86_64-unknown-linux-musl/release/deps/art* 
RUN rm src/*.rs 
COPY ./src ./src 
RUN cargo build --release

# Run
FROM alpine:latest 
WORKDIR /
# FROM scratch
# RUN apk --update add libressl-dev
COPY --from=cargo-build /usr/src/art/target/x86_64-unknown-linux-musl/release/art . 
CMD ["/art"]
