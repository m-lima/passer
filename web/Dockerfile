## Rust
FROM rust as wasm
WORKDIR /src

# wasm-pack
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

# Build
COPY wasm/passer .
RUN wasm-pack build

## Typescript
FROM node
WORKDIR /src

# Dependencies
COPY --from=wasm /src/pkg wasm/passer/pkg
COPY package.json package-lock.json tsconfig.json config-overrides.js ./
RUN npm install

# Build
COPY src src
COPY public public
RUN npm run build
