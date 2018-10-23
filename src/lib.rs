#![recursion_limit="128"]
#![feature(futures_api, pin, arbitrary_self_types)]
#![warn(unreachable_pub)]

#[macro_use]
extern crate stdweb;

#[macro_use]
extern crate stdweb_derive;

#[macro_use]
extern crate lazy_static;

extern crate discard;
extern crate futures_core;
extern crate futures_channel;
extern crate futures_util;
extern crate futures_signals;
extern crate pin_utils;


mod macros;
mod callbacks;
mod operations;
mod dom_operations;
mod dom;

pub use dom::*;
pub mod traits;
pub mod animation;

pub use stdweb::web::HtmlElement;

pub mod events {
	pub use stdweb::web::event::*;
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
