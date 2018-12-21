#[macro_use]
extern crate dominator;
#[macro_use]
extern crate lazy_static;

use std::sync::Arc;
use futures_signals::signal::{Mutable, SignalExt};
use dominator::Dom;
use dominator::events::ClickEvent;


struct State {
    counter: Mutable<i32>,
}

impl State {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            counter: Mutable::new(0),
        })
    }

    fn render(state: Arc<Self>) -> Dom {
        // Define CSS styles
        lazy_static! {
            static ref ROOT_CLASS: String = class! {
                .style("display", "inline-block")
                .style("background-color", "black")
                .style("padding", "10px")
            };

            static ref TEXT_CLASS: String = class! {
                .style("color", "white")
                .style("font-weight", "bold")
            };

            static ref BUTTON_CLASS: String = class! {
                .style("display", "block")
                .style("width", "100px")
                .style("margin", "5px")
            };
        }

        // Create the DOM nodes
        html!("div", {
            .class(&*ROOT_CLASS)

            .children(&mut [
                html!("div", {
                    .class(&*TEXT_CLASS)
                    .text_signal(state.counter.signal().map(|x| format!("Counter: {}", x)))
                }),

                html!("button", {
                    .class(&*BUTTON_CLASS)
                    .text("Increase")
                    .event(clone!(state => move |_: ClickEvent| {
                        // Increment the counter
                        state.counter.replace_with(|x| *x + 1);
                    }))
                }),

                html!("button", {
                    .class(&*BUTTON_CLASS)
                    .text("Decrease")
                    .event(clone!(state => move |_: ClickEvent| {
                        // Decrement the counter
                        state.counter.replace_with(|x| *x - 1);
                    }))
                }),

                html!("button", {
                    .class(&*BUTTON_CLASS)
                    .text("Reset")
                    .event(clone!(state => move |_: ClickEvent| {
                        // Reset the counter to 0
                        state.counter.set_neq(0);
                    }))
                }),
            ])
        })
    }
}


fn main() {
    let state = State::new();

    dominator::append_dom(&dominator::body(), State::render(state));
}
