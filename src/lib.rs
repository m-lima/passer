use wasm_bindgen::prelude::*;

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
pub struct Secret {
    key: Vec<u8>,
    payload: Vec<u8>,
}

#[wasm_bindgen]
impl Secret {
    #[wasm_bindgen(constructor)]
    pub fn new(key: &str, payload: &[u8]) -> Result<Secret, JsValue> {
        Ok(Self {
            key: base64::decode(key.as_bytes())
                .map_err(|_| Error::FailedToParseKey.into_js_value())?,
            payload: Vec::from(payload),
        })
    }

    fn new_inner(key: Vec<u8>, payload: Vec<u8>) -> Self {
        Self { key, payload }
    }

    pub fn key(&self) -> Result<js_sys::Uint8Array, JsValue> {
        unsafe {
            Ok(js_sys::Uint8Array::view(
                base64::encode(&self.key).as_bytes(),
            ))
        }
    }

    pub fn payload(&self) -> js_sys::Uint8Array {
        unsafe { js_sys::Uint8Array::view(&self.payload) }
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

    if let Ok(cipher_text) = cipher.encrypt(nonce, secret.as_bytes()) {
        Ok(Secret::new_inner(
            [&key_bytes[..], &nonce_bytes[..]].concat(),
            cipher_text,
        ))
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
