use wasm_bindgen::prelude::*;
use std::sync::Arc;
use once_cell::sync::Lazy;
use futures::stream::StreamExt;
use futures_signals::map_ref;
use futures_signals::signal::{Mutable, SignalExt};
use futures_signals::signal_vec::MutableVec;
use gloo_timers::future::IntervalStream;
use dominator::{Dom, html, clone, events, class};
use dominator::traits::AnimatedSignalVec;
use dominator::animation::{easing, Percentage, MutableAnimation, AnimatedMapBroadcaster};


struct Bar {
    color: u32,
    wave_animation: MutableAnimation,
    hover_animation: MutableAnimation,
}

impl Bar {
    fn new(color: u32) -> Arc<Self> {
        Arc::new(Self {
            color,
            wave_animation: MutableAnimation::new(3000.0),
            hover_animation: MutableAnimation::new(300.0),
        })
    }

    fn render(bar: Arc<Bar>, insert_animation: AnimatedMapBroadcaster) -> Dom {
        static CLASS: Lazy<String> = Lazy::new(|| class! {
            .style("border-radius", "10px")
            .style("position", "relative")
            .style("border-style", "solid")
            .style("border-width", "5px")
        });

        let low: f64 = bar.color as f64;
        let high: f64 = (bar.color + 60) as f64;

        html!("div", {
            .class(&*CLASS)


            .future(bar.wave_animation.signal().for_each(clone!(bar => move |t| {
                let t: f64 = t.into_f64();

                // Automatically cycles back and forth between 0 and 1
                // This creates a wave effect
                if t == 0.0 {
                    bar.wave_animation.animate_to(Percentage::new(1.0));

                } else if t == 1.0 {
                    bar.wave_animation.animate_to(Percentage::new(0.0));
                }

                async {}
            })))


            // Animation when hovering over the Bar
            .event(clone!(bar => move |_: events::MouseEnter| {
                bar.hover_animation.animate_to(Percentage::new(1.0));
            }))

            .event(clone!(bar => move |_: events::MouseLeave| {
                bar.hover_animation.animate_to(Percentage::new(0.0));
            }))


            // These will animate when the Bar is inserted/removed
            .style_signal("left", insert_animation.signal()
                .map(|t| t.none_if(1.0).map(|t| easing::in_out(t, easing::cubic)))
                .map(|t| t.map(|t| format!("{}px", t.range_inclusive(100.0, 0.0)))))

            .style_signal("height", map_ref! {
                let insert = insert_animation.signal().map(|t| easing::in_out(t, easing::cubic)),

                let hover = bar.hover_animation.signal().map(|t| easing::out(t, easing::cubic)) =>

                // Animate the height between 5px and 15px when hovering
                // But if the Bar is being inserted/removed then it will interpolate to 0px
                Some(format!("{}px", insert.range_inclusive(0.0, hover.range_inclusive(5.0, 15.0))))
            })

            .style_signal("border-width", insert_animation.signal()
                .map(|t| t.none_if(1.0).map(|t| easing::in_out(t, easing::cubic)))
                .map(|t| t.map(|t| format!("{}px", t.range_inclusive(0.0, 5.0)))))


            // These will animate in a continuous wave-like pattern
            .style_signal("width", bar.wave_animation.signal()
                .map(|t| easing::in_out(t, easing::cubic))
                .map(|t| Some(format!("{}px", t.range_inclusive(167.0, 500.0)))))

            .style_signal("margin-left", bar.wave_animation.signal()
                .map(|t| easing::in_out(t, easing::cubic))
                .map(|t| Some(format!("{}px", t.range_inclusive(0.0, 20.0)))))

            .style_signal("background-color", bar.wave_animation.signal()
                .map(|t| easing::in_out(t, easing::cubic))
                .map(move |t|
                    Some(format!("hsl({}, {}%, {}%)",
                        t.range_inclusive(low, high),
                        t.range_inclusive(50.0, 100.0),
                        t.range_inclusive(50.0, 100.0)))))

            .style_signal("border-color", bar.wave_animation.signal()
                .map(|t| easing::in_out(t, easing::cubic))
                .map(move |t|
                    Some(format!("hsl({}, {}%, {}%)",
                        t.range_inclusive(high, low),
                        t.range_inclusive(100.0, 50.0),
                        t.range_inclusive(100.0, 50.0)))))
        })
    }
}


struct App {
    current_color: Mutable<u32>,
    bars: MutableVec<Arc<Bar>>,
}

impl App {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            current_color: Mutable::new(0),
            bars: MutableVec::new(),
        })
    }

    fn new_color(&self) -> u32 {
        // Cycles through the color hue spectrum
        self.current_color.replace_with(|x| (*x + 10) % 360)
    }

    fn new_bar(&self) {
        let mut lock = self.bars.lock_mut();

        // Limits the number of bars to 40
        if lock.len() >= 40 {
            lock.remove(0);
        }

        lock.push_cloned(Bar::new(self.new_color()));
    }

    fn render_bars(app: Arc<Self>) -> Dom {
        html!("div", {
            .children_signal_vec(app.bars.signal_vec_cloned()
                // Animates the Bar for 2000ms when inserting/removing
                .animated_map(2000.0, |bar, animation| Bar::render(bar, animation)))
        })
    }

    fn render(app: Arc<Self>) -> Dom {
        static CLASS: Lazy<String> = Lazy::new(|| class! {
            .style("display", "flex")
            .style("flex-direction", "row")
        });

        html!("div", {
            .class(&*CLASS)

            // Inserts a new colored bar every 500ms
            .future(IntervalStream::new(500).for_each(clone!(app => move |_| {
                app.new_bar();
                async {}
            })))

            .children(&mut [
                // Renders the bars twice, both columns will always be perfectly in sync
                Self::render_bars(app.clone()),
                Self::render_bars(app),
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
