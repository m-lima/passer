use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Secret {
    key: js_sys::Uint8Array,
    payload: js_sys::Uint8Array,
}

#[wasm_bindgen]
impl Secret {
    #[wasm_bindgen(constructor)]
    pub fn new(key: &[u8], payload: &[u8]) -> Self {
        unsafe {
            Self {
                key: js_sys::Uint8Array::view(key),
                payload: js_sys::Uint8Array::view(payload),
            }
        }
    }

    pub fn key_raw(&self) -> js_sys::Uint8Array {
        self.key.clone()
    }

    // pub fn key_decoded(&self) -> js_sys::Uint8Array {}

    pub fn payload(&self) -> js_sys::Uint8Array {
        self.payload.clone()
    }
}

pub enum Error {
    NothingToProcess,
    FailedToProcess,
    InvalidKey,
    InvalidNonce,
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
            Self::InvalidNonce => JsValue::from("INVALID_NONCE"),
            Self::FailedToParseKey => JsValue::from("FAILED_TO_PARSE_KEY"),
        }
    }
}

#[wasm_bindgen]
pub fn encrypt(secret: &str) -> Result<Secret, JsValue> {
    console_error_panic_hook::set_once();
    use aes_gcm::aead::{generic_array::GenericArray, Aead, NewAead};
    use rand::Rng;

    if secret.is_empty() {
        return Err(Error::NothingToProcess.into());
    }

    let key_bytes = rand::thread_rng().gen::<[u8; 32]>();
    let nonce_bytes = rand::thread_rng().gen::<[u8; 12]>();

    let key = GenericArray::from_slice(&key_bytes);
    let cipher = aes_gcm::Aes256Gcm::new(key);

    let nonce = GenericArray::from_slice(&nonce_bytes);

    let key64 = base64::encode([&key_bytes[..], &nonce_bytes[..]].concat());

    if let Ok(cipher_text) = cipher.encrypt(nonce, secret.as_bytes()) {
        Ok(Secret::new(key64.as_bytes(), &cipher_text))
    } else {
        Err(Error::FailedToProcess.into())
    }
}

#[wasm_bindgen]
pub fn decrypt(key64: &[u8], secret: &[u8]) -> Result<js_sys::Uint8Array, JsValue> {
    use aes_gcm::aead::generic_array::GenericArray;
    use aes_gcm::aead::Aead;
    use aes_gcm::aead::NewAead;

    let key_decoded = base64::decode(key64).map_err(|_| Error::FailedToParseKey.into_js_value())?;

    if key_decoded.len() != 32 + 12 {
        return Err(Error::FailedToParseKey.into());
    }

    let key = GenericArray::from_slice(&key_decoded[..32]);
    let cipher = aes_gcm::Aes256Gcm::new(key);

    let nonce = GenericArray::from_slice(&key_decoded[32..]);

    if let Ok(cipher_text) = cipher.decrypt(nonce, secret) {
        unsafe { Ok(js_sys::Uint8Array::view(&cipher_text)) }
    } else {
        Err(Error::FailedToProcess.into())
    }
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
