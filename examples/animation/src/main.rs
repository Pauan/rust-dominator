#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate dominator;
#[macro_use]
extern crate futures_signals;

use std::rc::Rc;
use futures_signals::signal::SignalExt;
use futures_signals::signal_vec::MutableVec;
use dominator::traits::*;
use dominator::Dom;
use dominator::events::{MouseOverEvent, MouseOutEvent};
use dominator::animation::{Percentage, easing};
use dominator::animation::{MutableAnimation, AnimatedMapBroadcaster};


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

            Ok(())
        })))

        .event(clone!(hover_animation => move |_: MouseOverEvent| {
            hover_animation.animate_to(Percentage::new(1.0));
        }))

        .event(clone!(hover_animation => move |_: MouseOutEvent| {
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
        js! {
            console.log("Dropping");
        }
    }
}

fn main() {
    let state = Rc::new(State {
        boxes: MutableVec::new_with_values(vec![0]),
    });

    let mut color = 10;

    let f = clone!(state => move || {
        if state.boxes.len() >= 40 {
            state.boxes.remove(0);
        }

        state.boxes.push(color % 360);
        color += 10;
    });

    let _timer_id = js!(
        return setInterval(function () {
            @{f}();
        }, 500);
    );

    /*dominator::append_dom(&body,
        html!("button", {
            event(clone!(state => move |_: ClickEvent| {
                js! { @(no_return)
                    clearInterval(@{&timer_id});
                }

                state.boxes.clear();
            }));

            children(&mut [
                text("Clear all animations")
            ]);
        })
    );*/

    for _ in 0..1 {
        dominator::append_dom(&dominator::body(),
            html!("div", {
                .style("display", "flex")

                .children(&mut [
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
}
