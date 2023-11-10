# Build
FROM docker.io/rust:1.73.0-bookworm as rust
WORKDIR /src
COPY . .
RUN cargo build --release

# Pack
FROM docker.io/debian:stable-20231009-slim
COPY --from=rust /src/target/release/passer /usr/bin/passer
EXPOSE 80

ENV CLICOLOR_FORCE 1

ENTRYPOINT ["passer"]
