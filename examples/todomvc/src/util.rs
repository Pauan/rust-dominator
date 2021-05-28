use wasm_bindgen::prelude::*;
use web_sys::{window, Storage};


pub fn local_storage() -> Storage {
    window().unwrap_throw().local_storage().unwrap_throw().unwrap_throw()
}

#[inline]
pub fn trim(input: &str) -> Option<&str> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        None

    } else {
        Some(trimmed)
    }
}
