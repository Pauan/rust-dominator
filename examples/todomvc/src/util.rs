use wasm_bindgen::prelude::*;
use web_sys::{window, Storage};


pub fn local_storage() -> Storage {
    window().unwrap_throw().local_storage().unwrap_throw().unwrap_throw()
}

// TODO make this more efficient
#[inline]
pub fn trim(input: &str) -> Option<String> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        None

    } else {
        Some(trimmed.to_owned())
    }
}
