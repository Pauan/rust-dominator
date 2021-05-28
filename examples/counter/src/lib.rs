use wasm_bindgen::prelude::*;
use std::sync::Arc;
use once_cell::sync::Lazy;
use futures_signals::signal::{Mutable, SignalExt};
use dominator::{Dom, class, html, clone, events};


struct App {
    counter: Mutable<i32>,
}

impl App {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            counter: Mutable::new(0),
        })
    }

    fn render(state: Arc<Self>) -> Dom {
        // Define CSS styles
        static ROOT_CLASS: Lazy<String> = Lazy::new(|| class! {
            .style("display", "inline-block")
            .style("background-color", "black")
            .style("padding", "10px")
        });

        static TEXT_CLASS: Lazy<String> = Lazy::new(|| class! {
            .style("color", "white")
            .style("font-weight", "bold")
        });

        static BUTTON_CLASS: Lazy<String> = Lazy::new(|| class! {
            .style("display", "block")
            .style("width", "100px")
            .style("margin", "5px")
        });

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

    let app = App::new();
    dominator::append_dom(&dominator::body(), App::render(app));

    Ok(())
}
