## Rust
FROM docker.io/rust:1.73.0-bookworm as wasm
WORKDIR /src

# wasm-pack
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build
COPY ../wasm .
RUN wasm-pack build

## Typescript
FROM docker.io/node:21.0.0-bookworm as web
WORKDIR /web

# Dependencies
COPY --from=wasm /src/pkg wasm
COPY package.json yarn.lock tsconfig.json config-overrides.js ./
RUN yarn install

# Build
COPY src src
COPY public public
COPY cfg/Config.standalone.ts src/Config.ts
RUN yarn build

## Pack
FROM docker.io/alpine:3.18.4
COPY --from=web /web/build /web/build
