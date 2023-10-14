## Rust
FROM rust as wasm
WORKDIR /src

# wasm-pack
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build
COPY wasm .
RUN wasm-pack build

## Typescript
FROM node as web
WORKDIR /web

# Dependencies
COPY --from=wasm /src/pkg wasm/pkg
COPY package.json yarn.lock tsconfig.json config-overrides.js ./
RUN yarn install

# Build
COPY src src
COPY public public
COPY cfg/Config.standalone.ts src/Config.ts
RUN yarn build

## Pack
FROM alpine
COPY --from=web /web/build /web/build
