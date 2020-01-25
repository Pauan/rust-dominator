use wasm_bindgen::prelude::*;
use std::rc::Rc;
use futures::future::ready;
use futures_signals::map_ref;
use futures_signals::signal::SignalExt;
use futures_signals::signal_vec::MutableVec;
use dominator::traits::*;
use dominator::{Dom, html, clone, events};
use dominator::animation::{easing, Percentage, MutableAnimation, AnimatedMapBroadcaster};


fn make_animated_box(value: u32, broadcaster: AnimatedMapBroadcaster) -> Dom {
    let animation = Rc::new(MutableAnimation::new(3000.0));

    let hover_animation = Rc::new(MutableAnimation::new(300.0));

    let low: f64 = value as f64;
    let high: f64 = (value + 60) as f64;

    html!("div", {
        .future(animation.signal().for_each(clone!(animation => move |x| {
            let x: f64 = x.into_f64();

            if x == 0.0 {
                animation.animate_to(Percentage::new(1.0));

            } else if x == 1.0 {
                animation.animate_to(Percentage::new(0.0));
            }

            ready(())
        })))

        .event(clone!(hover_animation => move |_: events::MouseEnter| {
            hover_animation.animate_to(Percentage::new(1.0));
        }))

        .event(clone!(hover_animation => move |_: events::MouseLeave| {
            hover_animation.animate_to(Percentage::new(0.0));
        }))

        .style("border-radius", "10px")

        .style_signal("width", animation.signal()
            .map(|t| easing::in_out(t, easing::cubic))
            .map(|t| Some(format!("{}px", t.range_inclusive(167.0, 500.0)))))

        .style("position", "relative")

        .style_signal("margin-left", animation.signal()
            .map(|t| t.invert())
            .map(|t| easing::in_out(t, easing::cubic))
            .map(|t| Some(format!("{}px", t.range_inclusive(20.0, 0.0)))))

        .style_signal("left", broadcaster.signal()
            .map(|t| easing::in_out(t, easing::cubic))
            .map(|t| Some(format!("{}px", t.range_inclusive(100.0, 0.0)))))

        .style_signal("height", map_ref! {
            let animation = broadcaster.signal().map(|t| easing::in_out(t, easing::cubic)),
            let hover = hover_animation.signal().map(|t| easing::out(t, easing::cubic)) =>
            Some(format!("{}px", animation.range_inclusive(0.0, hover.range_inclusive(5.0, 15.0))))
        })

        .style_signal("background-color", animation.signal()
            .map(|t| easing::in_out(t, easing::cubic))
            .map(move |t| Some(format!("hsl({}, {}%, {}%)",
                t.range_inclusive(low, high),
                t.range_inclusive(50.0, 100.0),
                t.range_inclusive(50.0, 100.0)))))

        .style("border-style", "solid")

        .style_signal("border-width", broadcaster.signal()
            .map(|t| easing::in_out(t, easing::cubic))
            .map(|t| Some(format!("{}px", t.range_inclusive(0.0, 5.0)))))

        .style_signal("border-color", animation.signal()
            .map(|t| easing::in_out(t, easing::cubic))
            .map(move |t| Some(format!("hsl({}, {}%, {}%)",
                t.range_inclusive(high, low),
                t.range_inclusive(100.0, 50.0),
                t.range_inclusive(100.0, 50.0)))))
    })
}


struct State {
    boxes: MutableVec<u32>,
}

impl Drop for State {
    fn drop(&mut self) {
        web_sys::console::log_1(&JsValue::from("Dropping"));
    }
}


// TODO move this into gloo
fn set_interval<F>(ms: i32, f: F) where F: FnMut() + 'static {
    let f = wasm_bindgen::closure::Closure::wrap(Box::new(f) as Box<dyn FnMut()>);

    web_sys::window()
        .unwrap_throw()
        .set_interval_with_callback_and_timeout_and_arguments_0(wasm_bindgen::JsCast::unchecked_ref(f.as_ref()), ms)
        .unwrap_throw();

    // TODO cleanup
    f.forget()
}


#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();


    let state = Rc::new(State {
        boxes: MutableVec::new_with_values(vec![0]),
    });

    let mut color = 10;

    let _timer_id = set_interval(500, clone!(state => move || {
        let mut lock = state.boxes.lock_mut();

        if lock.len() >= 40 {
            lock.remove(0);
        }

        lock.push(color % 360);
        color += 10;
    }));

    /*dominator::append_dom(&body,
        html!("button", {
            .event(clone!(state => move |_: ClickEvent| {
                js! { @(no_return)
                    clearInterval(@{&timer_id});
                }

                state.boxes.clear();
            }))

            .text("Clear all animations")
        })
    );*/

    for _ in 0..1 {
        dominator::append_dom(&dominator::body(),
            html!("div", {
                .style("display", "flex")

                .children(vec![
                    html!("div", {
                        .children_signal_vec(state.boxes.signal_vec()
                            .animated_map(2000.0, |value, t| {
                                make_animated_box(value, t)
                            }))
                    }),

                    html!("div", {
                        .children_signal_vec(state.boxes.signal_vec()
                            .animated_map(2000.0, |value, t| {
                                make_animated_box(value, t)
                            }))
                    }),
                ])
            })
        );
    }

    Ok(())
}
