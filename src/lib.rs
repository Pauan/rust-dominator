#![warn(unreachable_pub)]
//#![deny(warnings)]

#![cfg_attr(feature = "nightly", allow(incomplete_features))]
#![cfg_attr(feature = "nightly", feature(adt_const_params, generic_const_exprs))]


#[macro_use]
mod macros;
mod utils;
mod bindings;
mod callbacks;
mod operations;
mod dom;
mod fragment;

pub use web_sys::ShadowRootMode;
pub use dom::*;
pub use fragment::*;
pub mod traits;
pub mod animation;
pub mod routing;
pub mod events;
