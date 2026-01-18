#![deny(warnings, clippy::pedantic, clippy::all, rust_2018_idioms)]
#![allow(clippy::missing_errors_doc)]
// Allowed because it is wasm
#![allow(clippy::must_use_candidate)]

//! Provides encryption using Chacha20-Poly1305 in wasm
//!
//! # Typical flow:
//! ## Encryption
//! `Either<String | [u8]> -> InnerPack -> Serialize() -> Compress() -> Encrypt() -> Encrypted`
//! ## Decryption
//! `Encrypted -> Decrypt() -> Decompress() -> Deserialize() -> InnerPack -> Pack`
//! Pack is then accessible from JS through wasm bindgen

use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
type JString = js_sys::JsString;
#[cfg(not(target_arch = "wasm32"))]
type JString = String;

#[cfg(target_arch = "wasm32")]
type JVec = js_sys::Uint8Array;
#[cfg(not(target_arch = "wasm32"))]
type JVec = Vec<u8>;

pub enum Error {
    FailedToProcess,
    InvalidKey,
    FailedToParseKey,
}

impl Error {
    fn into_js_value(self) -> JsValue {
        self.into()
    }
}

impl From<Error> for JsValue {
    fn from(value: Error) -> Self {
        match value {
            Error::FailedToProcess => JsValue::from("FAILED_TO_PROCESS"),
            Error::InvalidKey => JsValue::from("INVALID_KEY"),
            Error::FailedToParseKey => JsValue::from("FAILED_TO_PARSE_KEY"),
        }
    }
}

#[wasm_bindgen]
pub struct Key {
    key: [u8; 32],
    nonce: [u8; 12],
}

#[wasm_bindgen]
impl Key {
    #[wasm_bindgen(constructor)]
    pub fn new(key_bytes: &[u8]) -> Result<Key, JsValue> {
        if key_bytes.len() != 44 {
            return Err(Error::InvalidKey.into_js_value());
        }

        let mut key = [0; 32];
        key.copy_from_slice(&key_bytes[..32]);

        let mut nonce = [0; 12];
        nonce.copy_from_slice(&key_bytes[..12]);

        Ok(Self { key, nonce })
    }

    #[wasm_bindgen]
    pub fn from_base64(key_str: &str) -> Result<Key, JsValue> {
        Self::new(
            &base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, key_str)
                .map_err(|_| Error::FailedToParseKey.into_js_value())?,
        )
    }

    #[wasm_bindgen]
    pub fn to_base64(&self) -> JString {
        let mut key_bytes = [0; 44];
        key_bytes[..32].copy_from_slice(&self.key);
        key_bytes[32..].copy_from_slice(&self.nonce);
        let out =
            base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, key_bytes);
        JString::from(out)
    }

    fn encrypt(&self, pack: SerdePack<'_>) -> Result<Encrypted, JsValue> {
        use chacha20poly1305::aead::Aead;

        let binary = pack.serialize();
        let compressed = miniz_oxide::deflate::compress_to_vec(&binary, 8);
        let cipher = <chacha20poly1305::ChaCha20Poly1305 as chacha20poly1305::KeyInit>::new(
            &self.key.into(),
        );

        cipher
            .encrypt(&self.nonce.into(), compressed.as_slice())
            .map(|payload| Encrypted::new(&payload))
            .map_err(|_| Error::FailedToProcess.into_js_value())
    }

    #[wasm_bindgen]
    pub fn encrypt_string(&self, name: &str, data: &str) -> Result<Encrypted, JsValue> {
        self.encrypt(SerdePack {
            plain_message: true,
            name,
            size: data
                .len()
                .try_into()
                .map_err(|_| Error::FailedToProcess.into_js_value())?,
            data: data.as_bytes(),
        })
    }

    #[wasm_bindgen]
    pub fn encrypt_file(&self, name: &str, data: &[u8]) -> Result<Encrypted, JsValue> {
        self.encrypt(SerdePack {
            plain_message: false,
            name,
            size: data
                .len()
                .try_into()
                .map_err(|_| Error::FailedToProcess.into_js_value())?,
            data,
        })
    }

    #[wasm_bindgen]
    pub fn decrypt(&self, payload: &[u8]) -> Result<Pack, JsValue> {
        let cipher = <chacha20poly1305::ChaCha20Poly1305 as chacha20poly1305::KeyInit>::new(
            &self.key.into(),
        );

        let decrypted = chacha20poly1305::aead::Aead::decrypt(&cipher, &self.nonce.into(), payload)
            .map_err(|_| Error::FailedToProcess.into_js_value())?;
        let decompressed = miniz_oxide::inflate::decompress_to_vec(&decrypted)
            .map_err(|_| Error::FailedToProcess.into_js_value())?;

        SerdePack::deserialize(&decompressed)
            .map(Pack::new)
            .ok_or_else(|| Error::FailedToProcess.into_js_value())
    }
}

#[wasm_bindgen]
pub struct Encrypted {
    payload: JVec,
}

impl Encrypted {
    fn new(payload: &[u8]) -> Self {
        let payload = JVec::from(payload);
        Self { payload }
    }
}

#[wasm_bindgen]
impl Encrypted {
    pub fn payload(&self) -> JVec {
        self.payload.clone()
    }
}

/// The pack contains the data that should be encrypted and the metadata about it
#[derive(Clone, Copy)]
struct SerdePack<'a> {
    plain_message: bool,
    name: &'a str,
    size: u32,
    data: &'a [u8],
}

impl<'a> SerdePack<'a> {
    fn serialize(&self) -> Vec<u8> {
        let mut vec = Vec::with_capacity(
            1 + (size_of::<usize>() + self.name.len())
                + size_of::<usize>()
                + (size_of::<usize>() + self.data.len()),
        );
        vec.push(if self.plain_message { u8::MAX } else { 0 });
        vec.extend_from_slice(&self.name.len().to_le_bytes());
        vec.extend_from_slice(self.name.as_bytes());
        vec.extend_from_slice(&self.size.to_le_bytes());
        vec.extend_from_slice(&self.data.len().to_le_bytes());
        vec.extend_from_slice(self.data);
        vec
    }

    fn deserialize(mut bytes: &'a [u8]) -> Option<Self> {
        let plain_message = (!bytes.is_empty()).then(|| bytes[0] > 0)?;
        bytes = &bytes[1..];

        let name = {
            let len = (bytes.len() >= size_of::<usize>()).then(|| {
                let mut number = [0; size_of::<usize>()];
                number.copy_from_slice(&bytes[..size_of::<usize>()]);
                usize::from_le_bytes(number)
            })?;
            bytes = &bytes[size_of::<usize>()..];
            let data = (bytes.len() >= len).then(|| &bytes[..len])?;
            bytes = &bytes[len..];
            str::from_utf8(data).ok()?
        };

        let size = (bytes.len() >= size_of::<u32>()).then(|| {
            let mut number = [0; size_of::<u32>()];
            number.copy_from_slice(&bytes[..size_of::<u32>()]);
            u32::from_le_bytes(number)
        })?;
        bytes = &bytes[size_of::<u32>()..];

        let data = {
            let len = (bytes.len() >= size_of::<usize>()).then(|| {
                let mut number = [0; size_of::<usize>()];
                number.copy_from_slice(&bytes[..size_of::<usize>()]);
                usize::from_le_bytes(number)
            })?;
            bytes = &bytes[size_of::<usize>()..];
            (bytes.len() >= len).then(|| &bytes[..len])?
        };

        Some(Self {
            plain_message,
            name,
            size,
            data,
        })
    }
}

/// The pack contains the data that should be encrypted and the metadata about it
///
/// This struct exist to create a wasm interface to the inner pack
#[wasm_bindgen]
pub struct Pack {
    plain_message: bool,
    name: JString,
    size: u32,
    data: JVec,
}

impl Pack {
    fn new(inner: SerdePack<'_>) -> Self {
        let SerdePack {
            plain_message,
            name,
            size,
            data,
        } = inner;

        let name = name.into();
        let data = data.into();

        Self {
            plain_message,
            name,
            size,
            data,
        }
    }
}

#[wasm_bindgen]
impl Pack {
    pub fn plain_message(&self) -> bool {
        self.plain_message
    }

    pub fn name(&self) -> JString {
        self.name.clone()
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn data(&mut self) -> JVec {
        self.data.clone()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn string_round_trip() {
        let key_bytes = (1..).take(44).collect::<Vec<u8>>();
        let key = super::Key::new(&key_bytes).unwrap();
        let encrypted = key.encrypt_string("foo", "bar").unwrap();
        let decrypted = key.decrypt(&encrypted.payload).unwrap();

        assert!(decrypted.plain_message);
        assert_eq!(decrypted.name, "foo");
        assert_eq!(decrypted.size, 3);
        assert_eq!(decrypted.data, b"bar");
    }

    #[test]
    fn data_round_trip() {
        let key_bytes = (1..).take(44).collect::<Vec<u8>>();
        let key = super::Key::new(&key_bytes).unwrap();
        let encrypted = key.encrypt_file("foo", b"bar").unwrap();
        let decrypted = key.decrypt(&encrypted.payload).unwrap();

        assert!(!decrypted.plain_message);
        assert_eq!(decrypted.name, "foo");
        assert_eq!(decrypted.size, 3);
        assert_eq!(decrypted.data, b"bar");
    }
}
