use wasm_bindgen::prelude::*;
use web_sys::Url;
use futures_signals::signal::{Signal, SignalExt};
use dominator::routing;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Active,
    Completed,
    All,
}

impl Route {
    pub fn signal() -> impl Signal<Item = Self> {
        routing::url()
            .signal_ref(|url| Url::new(&url).unwrap_throw())
            .map(|url| {
                match url.hash().as_str() {
                    "#/active" => Route::Active,
                    "#/completed" => Route::Completed,
                    _ => Route::All,
                }
            })
    }

    pub fn url(&self) -> &'static str {
        match self {
            Route::Active => "#/active",
            Route::Completed => "#/completed",
            Route::All => "#/",
        }
    }
}
