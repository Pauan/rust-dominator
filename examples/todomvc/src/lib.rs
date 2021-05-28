use wasm_bindgen::prelude::*;
use crate::app::App;

mod util;
mod todo;
mod app;

#[wasm_bindgen(start)]
pub fn main_js() {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();

    dominator::append_dom(&dominator::get_id("app"), App::render(App::deserialize()));
}
