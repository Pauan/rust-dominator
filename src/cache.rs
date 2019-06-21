use std::cell::RefCell;
use std::collections::BTreeMap;
use js_sys::JsString;

thread_local! {
    // TODO is it possible to avoid the RefCell ?
    static CACHE: RefCell<BTreeMap<String, JsString>> = RefCell::new(BTreeMap::new());
}

// TODO make this more efficient
pub(crate) fn intern(x: &str) -> JsString {
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();

        match cache.get(x) {
            Some(value) => value.clone(),
            None => {
                let js = JsString::from(x);
                cache.insert(x.to_owned(), js.clone());
                js
            },
        }
    })
}


#[doc(hidden)]
pub fn debug_cache() {
    CACHE.with(|cache| {
        let cache = cache.borrow();
        web_sys::console::log_1(&wasm_bindgen::JsValue::from(format!("{:#?}", cache.keys())));
    });
}
