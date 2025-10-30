# Passer

[![Github](https://github.com/m-lima/passer/actions/workflows/check.yml/badge.svg)](https://github.com/m-lima/passer/actions/workflows/check.yml)

Encrypt files locally and share them securely

## Encyption flow

### Content is sent to the browser

![Step 1](https://raw.githubusercontent.com/m-lima/passer/master/web/src/img/passer-flow-enc-1.svg)

This happens locally without any upload

### Content is immediately encrypted

![Step 2](https://raw.githubusercontent.com/m-lima/passer/master/web/src/img/passer-flow-enc-2.svg)

A 256-bit key is generated and used for encryption as soon as content is loaded

### Encrypted content is uploaded

![Step 3](https://raw.githubusercontent.com/m-lima/passer/master/web/src/img/passer-flow-enc-3.svg)

The key stays behind in the browser and only a stream of encrypted bytes is sent

### Server assigns a unique 256-bit identifier

![Step 4](https://raw.githubusercontent.com/m-lima/passer/master/web/src/img/passer-flow-enc-4.svg)

The server only knows of the identifier and which encrypted bytes it refers to

### The browser returns both the identifier and the key

![Step 5](https://raw.githubusercontent.com/m-lima/passer/master/web/src/img/passer-flow-enc-5.svg)

Both are needed to access and decrypt the content. The server will delete the data after first download or if it expires

## Decryption flow

### An identifier is requested

![Step 1](https://raw.githubusercontent.com/m-lima/passer/master/web/src/img/passer-flow-dec-1.svg)

The 256-bit identifier only refers to a pack of encrypted bytes in the server

### The browser queries the server for the identifier

![Step 2](https://raw.githubusercontent.com/m-lima/passer/master/web/src/img/passer-flow-dec-2.svg)

### Encrypted content is downloaded

![Step 3](https://raw.githubusercontent.com/m-lima/passer/master/web/src/img/passer-flow-dec-3.svg)

The server immediately deletes the data and the browser owns the only copy

### The decryption key is provided

![Step 4](https://raw.githubusercontent.com/m-lima/passer/master/web/src/img/passer-flow-dec-4.svg)

The 256-bit key is used to decrypt the data locally

### The decrypted data is kept loaded in the browser

![Step 5](https://raw.githubusercontent.com/m-lima/passer/master/web/src/img/passer-flow-dec-5.svg)

### The decrypted content can be saved

![Step 6](https://raw.githubusercontent.com/m-lima/passer/master/web/src/img/passer-flow-dec-6.svg)

Being the only copy of the decrypted data, anything that is not downloaded is deleted

## Building

### Required tools

- cargo
- wasm-pack
- yarn

### Build the server

```bash
$ cd <THIS_REPO>/server
$ cargo build --release
```

### Build the wasm module

```bash
$ cd <THIS_REPO>/web/wasm
$ wasm-pack build --release
```

### Build the webpage

```bash
$ cd <THIS_REPO>/web
$ yarn install
$ yarn build
```

## Deploying

There are two ways **passer** can be run:

- Self-hosted single (simplest)
- Separate servers

### Self-hosted

The server will provide the API and the hosting of the website.

**Note:** As of version `0.7.0`, **passer** does not support TLS and is expected to be hosted behind
reverse proxy handling TLS

#### Update the API reference

```bash
$ cd <THIS_REPO>/web
$ cp cfg/Config.bundle.ts src/Config.ts
```

#### Build all the components

Follow [the build instructions](#building)

#### Start the server

```bash
$ cd <THIS_REPO>/server
$ cargo run --release --web-path ../web/build --port 3030
```

#### Access the server

Navigate to http://localhost:3030

### Separate servers

The server will only provide the API. Some other serve must provide the webpage, such as nginx or
webpack-dev-server for development

**Note:** As of version `0.7.0`, **passer** does not support TLS and is expected to be hosted behind
reverse proxy handling TLS

#### Update the API reference

```bash
$ cd <THIS_REPO>/web
$ cp cfg/Config.standalone.ts src/Config.ts
```

Replace the entry `export const API = '<YOUR_BACKEND>'` with the location of where the API
will be served.

#### Build all the components

Follow [the build instructions](#building)

#### Start the API

```bash
$ cd <THIS_REPO>/server
$ cargo run --release --port 3030
```

#### Start the webpage server

In this case, we will use webpack-dev-server as a quick demonstration

```bash
$ cd <THIS_REPO>/web
$ yarn start
```
