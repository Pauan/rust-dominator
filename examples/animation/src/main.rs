#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate dominator;
#[macro_use]
extern crate signals;

use stdweb::traits::*;
use stdweb::web::{document, HtmlElement};
use stdweb::web::event::{MouseOverEvent, MouseOutEvent};
use signals::signal::Signal;
use signals::signal_vec::unsync::MutableVec;
use dominator::traits::*;
use dominator::Dom;
use dominator::animation::{Percentage, easing};
use dominator::animation::unsync::MutableAnimation;


fn make_animated_box<A>(value: u32, t: A) -> Dom where A: Signal<Item = Percentage> + Clone + 'static {
    let animation = MutableAnimation::new(3000.0);

    let hover_animation = MutableAnimation::new(300.0);

    let low: f64 = value as f64;
    let high: f64 = (value + 60) as f64;

    html!("div", {
        future(animation.signal().for_each(clone!(animation => move |x| {
            let x: f64= x.into();

            if x == 0.0 {
                animation.animate_to(Percentage::new(1.0));

            } else if x == 1.0 {
                animation.animate_to(Percentage::new(0.0));
            }

            Ok(())
        })));

        event(clone!(hover_animation => move |_: MouseOverEvent| {
            hover_animation.animate_to(Percentage::new(1.0));
        }));

        event(clone!(hover_animation => move |_: MouseOutEvent| {
            hover_animation.animate_to(Percentage::new(0.0));
        }));

        style("border-radius", "10px");

        style("width", animation.signal()
            .map(|t| easing::in_out(t, easing::cubic))
            .map(|t| Some(format!("{}px", t.range_inclusive(167.0, 500.0))))
            .dynamic());

        style("position", "relative");

        style("margin-left", animation.signal()
            .map(|t| t.invert())
            .map(|t| easing::in_out(t, easing::cubic))
            .map(|t| Some(format!("{}px", t.range_inclusive(20.0, 0.0))))
            .dynamic());

        style("left", t.clone()
            .map(|t| easing::in_out(t, easing::cubic))
            .map(|t| Some(format!("{}px", t.range_inclusive(100.0, 0.0))))
            .dynamic());

        style("height",
            map_cloned! {
                let animation = t.clone().map(|t| easing::in_out(t, easing::cubic)),
                let hover = hover_animation.signal().map(|t| easing::out(t, easing::cubic)) =>
                Some(format!("{}px", animation.range_inclusive(0.0, hover.range_inclusive(5.0, 15.0))))
            }
            .dynamic());

        style("background-color", animation.signal()
            .map(|t| easing::in_out(t, easing::cubic))
            .map(move |t| Some(format!("hsl({}, {}%, {}%)",
                t.range_inclusive(low, high),
                t.range_inclusive(50.0, 100.0),
                t.range_inclusive(50.0, 100.0))))
            .dynamic());

        style("border-style", "solid");

        style("border-width", t.map(|t| easing::in_out(t, easing::cubic))
            .map(|t| Some(format!("{}px", t.range_inclusive(0.0, 5.0))))
            .dynamic());

        style("border-color", animation.signal()
            .map(|t| easing::in_out(t, easing::cubic))
            .map(move |t| Some(format!("hsl({}, {}%, {}%)",
                t.range_inclusive(high, low),
                t.range_inclusive(100.0, 50.0),
                t.range_inclusive(100.0, 50.0))))
            .dynamic());
    })
}


#[derive(Clone)]
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
    let state = State {
        boxes: MutableVec::new_with_values(vec![0]),
    };

    // TODO this should be in stdweb
    let body = document().query_selector("body").unwrap().unwrap();

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

    for _ in 0..2 {
        dominator::append_dom(&body,
            html!("div", {
                style("display", "flex");

                children(&mut [
                    html!("div", {
                        children(state.boxes.signal_vec()
                            .animated_map(2000.0, |value, t| {
                                make_animated_box(value, t)
                            })
                            .dynamic());
                    }),

                    html!("div", {
                        children(state.boxes.signal_vec()
                            .animated_map(2000.0, |value, t| {
                                make_animated_box(value, t)
                            })
                            .dynamic());
                    }),
                ]);
            })
        );
    }
}
