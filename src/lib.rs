use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    pub fn alert(secret: &str);
}

#[wasm_bindgen]
pub fn encrypt(secret: &str) {
    alert(&format!("Yoo {}!", secret));
}
