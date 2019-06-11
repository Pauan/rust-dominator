use wasm_bindgen::prelude::*;
use std::sync::Arc;
use lazy_static::lazy_static;
use futures_signals::signal::{Mutable, SignalExt};
use dominator::{Dom, class, html, clone, events};


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
                    .event(clone!(state => move |_: events::Click| {
                        // Increment the counter
                        state.counter.replace_with(|x| *x + 1);
                    }))
                }),

                html!("button", {
                    .class(&*BUTTON_CLASS)
                    .text("Decrease")
                    .event(clone!(state => move |_: events::Click| {
                        // Decrement the counter
                        state.counter.replace_with(|x| *x - 1);
                    }))
                }),

                html!("button", {
                    .class(&*BUTTON_CLASS)
                    .text("Reset")
                    .event(clone!(state => move |_: events::Click| {
                        // Reset the counter to 0
                        state.counter.set_neq(0);
                    }))
                }),
            ])
        })
    }
}


#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();


    let state = State::new();

    dominator::append_dom(&dominator::body(), State::render(state));

    Ok(())
}
