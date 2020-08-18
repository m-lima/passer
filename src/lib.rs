use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn encrypt(key: &[u8], nonce: &[u8], secret: &str) -> js_sys::Uint8Array {
    console_error_panic_hook::set_once();

    use aes_gcm::aead::generic_array::GenericArray;
    use aes_gcm::aead::Aead;
    use aes_gcm::aead::NewAead;

    let key = GenericArray::from_slice(key);
    let cipher = aes_gcm::Aes256Gcm::new(key);

    let nonce = GenericArray::from_slice(nonce);
    unsafe {
        js_sys::Uint8Array::view(
            &cipher
                .encrypt(nonce, secret.as_bytes())
                .expect("encryption failure"),
        )
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
                "Yoooo"
            )
            .length()
                > 0
        );
    }
}
