# Build
FROM rust as rust
WORKDIR /src
COPY . .
RUN cargo build --release

# Pack
FROM debian:stable-slim
COPY --from=rust /src/target/release/passer /usr/bin/passer
EXPOSE 80

ENTRYPOINT ["passer"]
