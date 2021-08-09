#![warn(unreachable_pub)]
#![deny(warnings)]
#![cfg_attr(feature = "nightly", allow(incomplete_features))]
#![cfg_attr(feature = "nightly", feature(const_generics))]

//! Zero-cost ultra-high-performance declarative DOM library using FRP signals for Rust! \

//! # Getting Started
//! Can be used as a standalone DOM rendering library but comes into its own when paired with
//! [futures-signals](https://docs.rs/futures-signals/). The best place to start is the
//! [Signals tutorial](https://docs.rs/futures-signals/*/futures_signals/tutorial/index.html)
//! and then become acquainted with [`DomBuilder<A>`](crate::dom::DomBuilder) and [`html!`](crate::html!).
//! Idiomatic use is:
//! - Create `struct` components that own data
//! - Wrap any data that can change in a [`Mutable<T>`](https://docs.rs/futures-signals/*/futures_signals/signal/struct.Mutable.html),
//! [`MutableVec<T>`](https://docs.rs/futures-signals/*/futures_signals/signal_vec/struct.MutableVec.html), or
//! [`MutableBTreeMap<K, V>`](https://docs.rs/futures-signals/*/futures_signals/signal_map/struct.MutableBTreeMap.html)
//! - Create render functions for each `struct` component
//! - Avoid local state in the DOM: all flags, triggers, data etc. for rendering should be stored in the `struct`
//! - Call the render functions for each component in a chain all the way down
//!
//! `dominator` handles any necessary DOM updates on changes to the data. A basic app looks something like the
//! below, but refer to the [examples](https://github.com/Pauan/rust-dominator/tree/master/examples) for a
//! comprehensive set of best practices.
//! ```
//! use dominator::{events::Click, html, Dom};
//! use futures_signals::signal::Mutable;
//! use wasm_bindgen::prelude::*;
//!
//! // Top level App component
//! struct App {
//!     name: String,
//!     info: Info, // Info sub-component
//! }
//!
//! impl App {
//!     fn new(msg: &str) -> Self {
//!         Self {
//!             name: String::from("Dominator App"),
//!             info: Info::new(msg),
//!         }
//!     }
//!     fn render(app: Self) -> Dom {
//!         html!("div", {
//!             .text(&app.name)
//!             // Call sub-component rendering functions
//!             .child(Info::render(&app.info))
//!         })
//!     }
//! }
//!
//! // Info sub-component
//! struct Info {
//!     msg: Mutable<String>, // String value that can change over time
//! }
//!
//! impl Info {
//!     fn new(msg: &str) -> Self {
//!         Self {
//!             msg: Mutable::new(String::from(msg)),
//!         }
//!     }
//!     fn render(info: &Self) -> Dom {
//!         html!("button", {
//!             // Text will automatically update on any changes to `msg`
//!             .text_signal(info.msg.signal_cloned())
//!             .event({
//!                 // Clone due to move + 'static
//!                 let msg = info.msg.clone();
//!                 // Update `msg` when clicked
//!                 move |_: Click| msg.set(String::from("Clicked"))
//!             })
//!         })
//!     }
//! }
//!
//! #[wasm_bindgen(start)]
//! pub fn main_js() -> Result<(), JsValue> {
//!     let app = App::new("Hello world");
//!     dominator::append_dom(&dominator::body(), App::render(app));
//!     Ok(())
//! }
//! ```

//! ## Handling `'static` web apis
//! There are lots of `'static` lifetime requirements for web apis and therefore calls to `move` and `clone`: so
//! [`Arc<T>`](https://doc.rust-lang.org/std/sync/struct.Arc.html) and [`Rc<T>`](https://doc.rust-lang.org/std/rc/struct.Rc.html)
//! quickly come in handy. Which to use? As a general rule [`Arc<T>`](https://doc.rust-lang.org/std/sync/struct.Arc.html)
//! for rust types unless they [`!Send`](https://doc.rust-lang.org/nomicon/send-and-sync.html):
//! currently on WASM rust uses [single threaded primitives](https://github.com/rust-lang/rust/blob/1f94abcda6884893d4723304102089198caa0839/library/std/src/sys/wasm/mod.rs)
//! so there is no cost to using atomics and you will be ready for multi-threaded wasm;
//! use [`Rc<T>`](https://doc.rust-lang.org/std/rc/struct.Rc.html) for any JS values which are not thread safe and likely never will be.
//! \
//! \
//! There is the [`clone!`](crate::clone!) macro which is a nice shorthand for the many calls to `.clone()`.
//! \
//! \
//! [`MutableVec<T>`](https://docs.rs/futures-signals/*/futures_signals/signal_vec/struct.MutableVec.html) and
//! [`MutableBTreeMap<K, V>`](https://docs.rs/futures-signals/*/futures_signals/signal_map/struct.MutableBTreeMap.html) do
//! not `impl Clone` so if you want to make them cloneable you will need to wrap them in an [`Arc<T>`](https://doc.rust-lang.org/std/sync/struct.Arc.html)
//! or [`Rc<T>`](https://doc.rust-lang.org/std/rc/struct.Rc.html).

//! ## Clone and [`Mutable<T>`](https://docs.rs/futures-signals/*/futures_signals/signal/struct.Mutable.html)
//! [`Mutable<T>`](https://docs.rs/futures-signals/*/futures_signals/signal/struct.Mutable.html)
//! uses [`Arc<T>`](https://doc.rust-lang.org/std/sync/struct.Arc.html) internally so cloning it
//! calls [`Arc::clone`](https://doc.rust-lang.org/std/sync/struct.Arc.html#impl-Clone) and
//! will create another pointer to the same allocation. This can have subtle consequences when cloning any
//! structs with [`Mutable<T>`](https://docs.rs/futures-signals/*/futures_signals/signal/struct.Mutable.html) in.
//! ```
//! #[derive(Clone)]
//! struct Info {
//!     msg: Mutable<String>,
//! }
//!
//! let item = Info {
//!     msg: Mutable::new(String::from("original")),
//! };
//!
//! let item_clone = item.clone();
//!
//! // All updates to `msg` on the clone will update it on the original
//! item_clone.msg.set(String::from("both changed"));
//!
//! assert_eq!(item.msg.get_cloned(), item_clone.msg.get_cloned());
//!
//! // All updates to `msg` on the original will update it on the clone
//! item.msg.set(String::from("changed again"));
//!
//! assert_eq!(item.msg.get_cloned(), item_clone.msg.get_cloned());
//! ```

//! # Mixins
//! `dominator` has a great way of creating reusable functionalities and components: create a function with
//! the signature `DomBuilder<A> -> DomBuilder<A>`.
//! ```
//! fn mixin<A>(builder: DomBuilder<A>) -> DomBuilder<A> {
//!     // Do some stuff with the builder and then return it
//!     builder
//! }
//! ```
//! It can then be called from within the [`html!`](crate::html!) macro using the [`apply`](crate::dom::DomBuilder::apply) or
//! [`apply_if`](crate::dom::DomBuilder::apply_if) methods:
//! ```
//! html!("div", {
//!     .apply(mixin)
//!     .apply_if(true, mixin)
//! })
//! ```

//! ## js Bundler
//! `dominator` works really well when paired with the [rollup.js](https://www.rollupjs.org/guide/en/) and the
//! [rust rollup plugin](https://github.com/wasm-tool/rollup-plugin-rust). See the
//! [examples](https://github.com/Pauan/rust-dominator/tree/master/examples) folders for how to get setup.

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
