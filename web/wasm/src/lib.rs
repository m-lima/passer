#![deny(warnings, clippy::pedantic, clippy::all, rust_2018_idioms)]
#![allow(clippy::missing_errors_doc)]
// Allowed because it is wasm
#![allow(clippy::must_use_candidate)]

//! Provides encryption using AES-GCM in wasm
//!
//! # Typical flow:
//! ## Encryption
//! `Either<String | [u8]> -> InnerPack -> Serialize() -> Compress() -> Encrypt() -> Encrypted`
//! ## Decryption
//! `Encrypted -> Decrypt() -> Decompress() -> Deserialize() -> InnerPack -> Pack`
//! Pack is then accessible from JS through wasm bindgen

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

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
    cipher: aes_gcm::Aes256Gcm,
    key: [u8; 44],
}

#[wasm_bindgen]
impl Key {
    #[wasm_bindgen(constructor)]
    pub fn new(key_bytes: &[u8]) -> Result<Key, JsValue> {
        use aes_gcm::aead::{generic_array::GenericArray, KeyInit};

        if key_bytes.len() != 44 {
            return Err(Error::InvalidKey.into_js_value());
        }

        let mut key = [0; 44];
        key.copy_from_slice(key_bytes);

        Ok(Self {
            cipher: aes_gcm::Aes256Gcm::new(GenericArray::from_slice(&key_bytes[..32])),
            key,
        })
    }

    #[wasm_bindgen]
    pub fn from_string(key_str: &str) -> Result<Key, JsValue> {
        Self::new(
            &base64::decode_config(key_str.as_bytes(), base64::URL_SAFE_NO_PAD)
                .map_err(|_| Error::FailedToParseKey.into_js_value())?,
        )
    }

    #[wasm_bindgen]
    pub fn to_string(&self) -> js_sys::JsString {
        base64::encode_config(&self.key[..], base64::URL_SAFE_NO_PAD).into()
    }

    fn encrypt(&self, pack: &SerdePack) -> Result<Encrypted, JsValue> {
        use aes_gcm::aead::{generic_array::GenericArray, Aead};

        let binary =
            bincode::serialize(&pack).map_err(|_| Error::FailedToProcess.into_js_value())?;
        let compressed = miniz_oxide::deflate::compress_to_vec(&binary, 8);

        Ok(Encrypted(
            self.cipher
                .encrypt(
                    GenericArray::from_slice(&self.key[32..]),
                    compressed.as_slice(),
                )
                .map_err(|_| Error::FailedToProcess.into_js_value())?,
        ))
    }

    #[wasm_bindgen]
    pub fn encrypt_string(&self, name: &str, data: &str) -> Result<Encrypted, JsValue> {
        let pack = {
            let data = Vec::from(data);
            let size = data.len();
            SerdePack {
                plain_message: true,
                name: name.into(),
                size,
                data,
            }
        };
        self.encrypt(&pack)
    }

    #[wasm_bindgen]
    pub fn encrypt_file(&self, name: &str, data: &[u8]) -> Result<Encrypted, JsValue> {
        let pack = SerdePack {
            plain_message: false,
            name: name.into(),
            size: data.len(),
            data: data.into(),
        };
        self.encrypt(&pack)
    }

    #[wasm_bindgen]
    pub fn decrypt(&self, payload: &[u8]) -> Result<Pack, JsValue> {
        use aes_gcm::aead::{generic_array::GenericArray, Aead};

        let decrypted = self
            .cipher
            .decrypt(GenericArray::from_slice(&self.key[32..]), payload)
            .map_err(|_| Error::FailedToProcess.into_js_value())?;
        let decompressed = miniz_oxide::inflate::decompress_to_vec(&decrypted)
            .map_err(|_| Error::FailedToProcess.into_js_value())?;

        bincode::deserialize(&decompressed)
            .map(Pack::new)
            .map_err(|_| Error::FailedToProcess.into_js_value())
    }
}

#[wasm_bindgen]
pub struct Encrypted(Vec<u8>);

#[wasm_bindgen]
impl Encrypted {
    pub fn payload(&self) -> js_sys::Uint8Array {
        unsafe { js_sys::Uint8Array::view(&self.0) }
    }
}

/// The pack contains the data that should be encrypted and the metadata about it
#[derive(Serialize, Deserialize)]
struct SerdePack {
    plain_message: bool,
    name: String,
    size: usize,
    data: Vec<u8>,
}

/// The pack contains the data that should be encrypted and the metadata about it
///
/// This struct exist to create a wasm interface to the inner pack
#[wasm_bindgen]
pub struct Pack {
    inner: SerdePack,
}

impl Pack {
    fn new(inner: SerdePack) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen]
impl Pack {
    pub fn plain_message(&self) -> js_sys::Boolean {
        self.inner.plain_message.into()
    }

    pub fn name(&self) -> js_sys::JsString {
        self.inner.name.clone().into()
    }

    pub fn size(&self) -> usize {
        self.inner.size
    }

    pub fn data(&self) -> js_sys::Uint8Array {
        unsafe { js_sys::Uint8Array::view(&self.inner.data) }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn string_round_trip() {
        let key_bytes = (1..).take(44).collect::<Vec<u8>>();
        let key = super::Key::new(&key_bytes).unwrap();
        let encrypted = key.encrypt_string("foo", "bar").unwrap();
        let decrypted = key.decrypt(&encrypted.0).unwrap().inner;
        assert!(decrypted.plain_message);
        assert_eq!(decrypted.name, "foo");
        assert_eq!(decrypted.data, Vec::from("bar"));
    }

    #[test]
    fn data_round_trip() {
        let key_bytes = (1..).take(44).collect::<Vec<u8>>();
        let key = super::Key::new(&key_bytes).unwrap();
        let encrypted = key.encrypt_file("foo", b"bar").unwrap();
        let decrypted = key.decrypt(&encrypted.0).unwrap().inner;
        assert!(!decrypted.plain_message);
        assert_eq!(decrypted.name, "foo");
        assert_eq!(decrypted.data, Vec::from("bar"));
    }
}
