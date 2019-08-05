#![recursion_limit="128"]
#![warn(unreachable_pub)]
#![deny(warnings)]


#[macro_use]
mod macros;
mod utils;
mod bindings;
mod callbacks;
mod operations;
mod dom;

pub use dom::*;
pub mod traits;
pub mod animation;
pub mod routing;
pub mod events;
