use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    pub fn alert(secret: &str);
}

fn encrypt_inner(key: &str, nonce: &str, secret: &str) -> Vec<u8> {
    use aes_gcm::aead::generic_array::GenericArray;
    use aes_gcm::aead::Aead;
    use aes_gcm::aead::NewAead;

    let key = GenericArray::from_slice(key.as_bytes());
    let cipher = aes_gcm::Aes256Gcm::new(key);

    let nonce = GenericArray::from_slice(nonce.as_bytes());
    cipher
        .encrypt(nonce, secret.as_bytes())
        .expect("encryption failure")
}

#[wasm_bindgen]
pub fn encrypt(key: &str, nonce: &str, secret: &str) {
    console_error_panic_hook::set_once();
    alert(&format!("{:?}", encrypt_inner(key, nonce, secret)));
}

#[cfg(test)]
mod tests {
    #[test]
    fn can_encrypt() {
        assert!(
            !super::encrypt_inner("12345678901234567890123456789012", "123456789012", "Yoooo")
                .is_empty()
        );
    }
}
