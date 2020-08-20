use wasm_bindgen::prelude::*;

pub enum Error {
    NothingToProcess,
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
            Self::NothingToProcess => JsValue::from("NOTHING_TO_PROCESS"),
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

    fn encrypt(&self, payload: &[u8]) -> Result<Encrypted, Error> {
        use aes_gcm::aead::{generic_array::GenericArray, Aead};

        Ok(Encrypted(
            self.cipher
                .encrypt(GenericArray::from_slice(&self.key[32..]), payload)
                .map_err(|_| Error::FailedToProcess)?,
        ))
    }

    fn decrypt(&self, payload: &[u8]) -> Result<Decrypted, Error> {
        use aes_gcm::aead::{generic_array::GenericArray, Aead};

        Ok(Decrypted(
            self.cipher
                .decrypt(GenericArray::from_slice(&self.key[32..]), payload)
                .map_err(|_| Error::FailedToProcess)?,
        ))
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
pub struct Decrypted(Vec<u8>);

#[wasm_bindgen]
impl Decrypted {
    pub fn payload(&self) -> js_sys::Uint8Array {
        unsafe { js_sys::Uint8Array::view(&self.0) }
    }
}

#[wasm_bindgen]
pub fn encrypt(key: &Key, payload: &[u8]) -> Result<Encrypted, JsValue> {
    if payload.is_empty() {
        return Err(Error::NothingToProcess.into());
    }

    key.encrypt(payload).map_err(Error::into_js_value)
}

#[wasm_bindgen]
pub fn encrypt_string(key: &Key, payload: &str) -> Result<Encrypted, JsValue> {
    encrypt(key, payload.as_bytes())
}

#[wasm_bindgen]
pub fn decrypt(key: &Key, payload: &[u8]) -> Result<Decrypted, JsValue> {
    if payload.is_empty() {
        return Err(Error::NothingToProcess.into());
    }

    key.decrypt(payload).map_err(Error::into_js_value)
}

#[cfg(test)]
mod tests {
    #[test]
    fn can_encrypt() {
        assert!(
            super::encrypt(
                b"12345678901234567890123456789012",
                b"123456789012",
                "Yo! This is secret!"
            )
            .unwrap()
            .length()
                > 0
        );
    }
}
