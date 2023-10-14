## Wasm
FROM rust as wasm
WORKDIR /src

# wasm-pack
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build
COPY web/wasm .
RUN wasm-pack build

## Typescript
FROM node as web
WORKDIR /web

# Dependencies
COPY --from=wasm /src/pkg wasm/pkg
COPY web/package.json \
     web/yarn.lock \
     web/tsconfig.json \
     web/config-overrides.js \
     ./
RUN yarn install

# Build
COPY web/src src
COPY web/public public
COPY web/cfg/Config.bundle.ts src/Config.ts
RUN yarn build

## Rust
FROM rust as rust
WORKDIR /src

# Build
COPY server .
RUN cargo build --release --features host-frontend

# Pack
FROM debian:stable-slim
COPY --from=rust /src/target/release/passer /usr/bin/passer
COPY --from=web /web/build /var/www
EXPOSE 80

ENTRYPOINT ["passer", "--web-path", "/var/www"]
