#[macro_use]
extern crate stdweb;

#[macro_use]
extern crate dominator;

#[macro_use]
extern crate futures_signals;

use std::rc::Rc;
use stdweb::web::{document, HtmlElement};
use stdweb::web::event::ClickEvent;
use stdweb::web::IParentNode;

use futures_signals::signal;
use futures_signals::signal_vec;
use futures_signals::signal::Signal;
use futures_signals::signal_vec::SignalVec;
use dominator::traits::*;
use dominator::{Dom, text};


fn main() {
    stylesheet!("div", {
        style("border", "5px solid black");
    });

    let foobar = class! {
        style("border-right", "10px solid purple");
    };

    /*let media_query = stylesheet!(format!("@media (max-width: 500px) .{}", foobar), {
        style("border-left", "10px solid teal");
    });*/

    let mut count = 0;

    let (sender_count, receiver_count) = signal::unsync::mutable(count);


    let mut width: u32 = 10;

    let (sender1, receiver1) = signal::unsync::mutable(width);
    let (sender2, receiver2) = signal::unsync::mutable(vec![width]);
    let (sender3, receiver3) = signal::unsync::mutable(vec![width]);
    let (text_sender, text_receiver) = signal::unsync::mutable(format!("{}", width));

    let (mut sender_elements, receiver_elements) = signal_vec::unsync::mutable();

    /*let style_width = receiver1.switch(move |x| {
        receiver2.clone().switch(move |y| {
            receiver3.clone().map(move |z| {
                Some(format!("{}px", x + y[0] + z[0]))
            })
        })
    });*/

    let style_width = map_clone! {
        let x: Rc<u32> = receiver1,
        let y: Rc<Vec<u32>> = receiver2,
        let z: Rc<Vec<u32>> = receiver3 =>
        Some(format!("{}px", *x + y[0] + z[0]))
    };


    let mut elements_index = 0;

    let mut increment = move || {
        elements_index += 1;
        elements_index
    };

    sender_elements.push((increment(), 1));
    sender_elements.push((increment(), 2));
    sender_elements.push((increment(), 3));
    sender_elements.push((increment(), 4));
    sender_elements.push((increment(), 5));
    sender_elements.push((increment(), 6));
    sender_elements.push((increment(), 7));


    dominator::append_dom(&document().query_selector("body").unwrap().unwrap(),
        html!("div", {
            style("border", "10px solid blue");
            children(&mut [
                text("Testing testing!!!"),

                text(text_receiver.dynamic()),

                text(receiver_count.map(|x| format!(" - {}", x)).dynamic()),

                html!("div", {
                    style("width", style_width.dynamic());
                    style("height", "50px");
                    style("background-color", "green");
                    event(move |event: ClickEvent| {
                        count += 1;
                        width += 5;

                        console!(log, &event);

                        sender1.set(width).unwrap();
                        sender2.set(vec![width]).unwrap();
                        sender3.set(vec![width]).unwrap();
                        text_sender.set(format!("{}", width)).unwrap();
                        sender_count.set(count).unwrap();
                        sender_elements.push((increment(), 8));
                        sender_elements.push((increment(), 0));
                        sender_elements.push((increment(), 5));
                        sender_elements.push((increment(), 9));
                    });
                    children(
                        receiver_elements
                            .filter_map(|(x, y)| {
                                if y > 2 {
                                    Some((x, y + 100))
                                } else {
                                    None
                                }
                            })
                            .sort_by(|&(_, a), &(_, b)| {
                                a.cmp(&b).reverse()
                            })
                            .map(|(x, y)| {
                                html!("div", {
                                    style("border", "5px solid red");
                                    style("width", "100px");
                                    style("height", "50px");
                                    children(&mut [
                                        text(format!("({}, {})", x, y))
                                    ]);
                                })
                            })
                            .dynamic()
                    );
                }),

                html!("div", {
                    style("width", "50px");
                    style("height", "50px");
                    style("background-color", "red");
                    class(&foobar, true);
                    children(&mut [
                        html!("div", {
                            style("width", "10px");
                            style("height", "10px");
                            style("background-color", "orange");
                        })
                    ]);
                }),

                Dom::with_state(Rc::new(vec![1, 2, 3]), |a| {
                    html!("div", {
                        style("width", "100px");
                        style("height", "100px");
                        style("background-color", "orange");
                        class("foo", true);
                        class("bar", false);
                        event(clone!({ a } move |event: ClickEvent| {
                            console!(log, &*a, &event);
                        }));
                    })
                }),

                html!("input", {
                    focused(true);
                }),
            ]);
        })
    );
}
