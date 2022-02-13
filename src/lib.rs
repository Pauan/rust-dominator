//! Zero-cost ultra-high-performance declarative DOM library using FRP signals for Rust!
//!
//! If you haven't used `dominator` before, it's highly recommended that you read the
//! [`futures_signals::tutorial`], which provides a guide to using this implementation of signals.
//!
//! # Examples
//!
//! A very simple counter example:
//!
//! ```
//! use wasm_bindgen::prelude::*;
//! use std::sync::Arc;
//! use futures_signals::signal::{Mutable, SignalExt};
//! use dominator::{Dom, clone, html, events};
//!
//! struct App {
//!     counter: Mutable<i32>,
//! }
//!
//! impl App {
//!     fn new() -> Arc<Self> {
//!         Arc::new(Self { counter: Mutable::new(0) })
//!     }
//!
//!     fn render(state: Arc<Self>) -> Dom {
//!         html!("div", {
//!             .children(&mut [
//!                 html!("div", {
//!                     .text_signal(state.counter.signal().map(|x| format!("Counter: {}", x)))
//!                 }),
//!                 html!("button", {
//!                     .text("+")
//!                     .event(clone!(state => move |_: events::Click| {
//!                         state.counter.replace_with(|x| *x + 1);
//!                     }))
//!                 }),
//!             ])
//!         })
//!     }
//! }
//!
//! #[wasm_bindgen(start)]
//! pub fn main_js() -> Result<(), JsValue> {
//!     let app = App::new();
//!     dominator::append_dom(&dominator::body(), App::render(app));
//!
//!     Ok(())
//! }
//! ```
#![warn(unreachable_pub)]

#[macro_use]
mod macros;
mod bindings;
mod callbacks;
mod dom;
mod operations;
mod utils;

pub use dom::*;
pub use web_sys::ShadowRootMode;
pub mod animation;
pub mod events;
pub mod routing;
pub mod traits;
