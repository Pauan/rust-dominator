use web_sys::{window, Storage};


pub fn local_storage() -> Storage {
    window().unwrap().local_storage().unwrap().unwrap()
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
