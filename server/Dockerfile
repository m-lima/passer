# Build
FROM rust as rust
WORKDIR /src
COPY . .
RUN cargo build --release

# Pack
FROM ubuntu
COPY --from=rust /src/target/release/passer /usr/bin/passer
EXPOSE 80

CMD ["passer"]
