#![recursion_limit="128"]

#[macro_use]
extern crate stdweb;

#[macro_use]
extern crate stdweb_derive;

#[macro_use]
extern crate lazy_static;

extern crate discard;
extern crate futures;
extern crate signals;


mod macros;
mod callbacks;
mod operations;
mod dom_operations;
mod dom;

pub use dom::*;
pub mod traits;
pub mod animation;


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
