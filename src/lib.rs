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

impl std::convert::Into<JsValue> for Error {
    fn into(self) -> JsValue {
        match self {
            Self::FailedToProcess => JsValue::from("FAILED_TO_PROCESS"),
            Self::InvalidKey => JsValue::from("INVALID_KEY"),
            Self::FailedToParseKey => JsValue::from("FAILED_TO_PARSE_KEY"),
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
        use aes_gcm::aead::{generic_array::GenericArray, NewAead};

        if key_bytes.len() != 44 {
            return Err(Error::InvalidKey.into_js_value());
        }

        let mut key = [0; 44];
        key.copy_from_slice(&key_bytes);

        Ok(Self {
            cipher: aes_gcm::Aes256Gcm::new(GenericArray::from_slice(&key_bytes[..32])),
            key,
        })
    }

    #[wasm_bindgen]
    pub fn from_string(key_str: &str) -> Result<Key, JsValue> {
        Self::new(
            &base64::decode(key_str.as_bytes())
                .map_err(|_| Error::FailedToParseKey.into_js_value())?,
        )
    }

    #[wasm_bindgen]
    pub fn to_string(&self) -> js_sys::JsString {
        base64::encode(&self.key[..]).into()
    }

    #[wasm_bindgen]
    pub fn encrypt(&self, pack: Pack) -> Result<Encrypted, JsValue> {
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
    pub fn decrypt(&self, payload: &[u8]) -> Result<Pack, JsValue> {
        use aes_gcm::aead::{generic_array::GenericArray, Aead};

        let decrypted = self
            .cipher
            .decrypt(GenericArray::from_slice(&self.key[32..]), payload)
            .map_err(|_| Error::FailedToProcess.into_js_value())?;
        let decompressed = miniz_oxide::inflate::decompress_to_vec(&decrypted)
            .map_err(|_| Error::FailedToProcess.into_js_value())?;

        bincode::deserialize(&decompressed).map_err(|_| Error::FailedToProcess.into_js_value())
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

#[wasm_bindgen]
#[derive(Serialize, Deserialize)]
pub struct Pack {
    plain_message: bool,
    name: String,
    size: usize,
    data: Vec<u8>,
}

#[wasm_bindgen]
impl Pack {
    #[wasm_bindgen]
    pub fn pack_string(name: &str, data: &str) -> Self {
        let data = Vec::from(data);
        let size = data.len();
        Self {
            plain_message: true,
            name: name.into(),
            size,
            data,
        }
    }

    #[wasm_bindgen]
    pub fn pack_file(name: &str, data: &[u8]) -> Self {
        Self {
            plain_message: false,
            name: name.into(),
            size: data.len(),
            data: data.into(),
        }
    }

    pub fn plain_message(&self) -> js_sys::Boolean {
        self.plain_message.into()
    }

    pub fn name(&self) -> js_sys::JsString {
        self.name.clone().into()
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn data(&self) -> js_sys::Uint8Array {
        unsafe { js_sys::Uint8Array::view(&self.data) }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn round_trip() {
        let key = super::Key::new(&[0; 44]).unwrap();
        let pack = super::Pack::pack_string("foo", "bar");
        let encrypted = key.encrypt(pack).unwrap();
        let decrypted = key.decrypt(&encrypted.0).unwrap();
        assert!(decrypted.plain_message);
        assert_eq!(decrypted.name, "foo");
        assert_eq!(decrypted.data, Vec::from("bar"));
    }
}
