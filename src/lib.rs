#![warn(unreachable_pub)]
#![deny(warnings)]

#![cfg_attr(feature = "nightly", allow(incomplete_features))]
#![cfg_attr(feature = "nightly", feature(const_generics))]


#[macro_use]
mod macros;
mod utils;
mod bindings;
mod callbacks;
mod operations;
mod dom;

pub use web_sys::ShadowRootMode;
pub use dom::*;
pub mod traits;
pub mod animation;
pub mod routing;
pub mod events;
