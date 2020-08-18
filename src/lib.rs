use wasm_bindgen::prelude::*;

// #[wasm_bindgen]
// extern "C" {
//     pub type Uint8Array;

//     #[wasm_bindgen(constructor)]
//     pub fn new_with_byte_offset_and_length(
//         buffer: &JsValue,
//         byte_offset: u32,
//         length: u32,
//     ) -> Uint8Array;
// }

// pub unsafe fn view(data: &[u8]) -> Uint8Array {
//     let buf = wasm_bindgen::memory();
//     let mem = buf.unchecked_ref::<WebAssembly::Memory>();
//     Uint8Array::new_with_byte_offset_and_length(
//         &mem.buffer(),
//         data.as_ptr() as u32,
//         data.len() as u32,
//     )
// }

// #[wasm_bindgen]
// pub struct Cipher {
//     offset: *const u8,
//     size: usize,
// }

// #[wasm_bindgen]
// impl Cipher {
//     pub fn from(bytes: Vec<u8>) -> Self {
//         Self {
//             offset: bytes.as_ptr(),
//             size: bytes.len(),
//         }
//     }

//     pub fn offet(&self) -> *const u8 {
//         self.offset
//     }

//     pub fn size(&self) -> usize {
//         self.size
//     }
// }

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
