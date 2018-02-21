#[macro_use]
extern crate stdweb;

#[macro_use]
extern crate stdweb_derive;

#[macro_use]
extern crate lazy_static;

extern crate futures;

mod dom;
pub use dom::*;

pub mod signal;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
