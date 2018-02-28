#![feature(trace_macros)]
#![feature(log_syntax)]

#[macro_use]
extern crate stdweb;

#[macro_use]
extern crate dominator;

#[macro_use]
extern crate signals;

use std::rc::Rc;
use stdweb::web::{document, HtmlElement};
use stdweb::web::event::ClickEvent;
use stdweb::web::IParentNode;

use signals::signal;
use signals::signal::Signal;
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

    let (sender_elements, receiver_elements) = signal::unsync::mutable(count);


    let mut width: u32 = 10;

    let (sender1, receiver1) = signal::unsync::mutable(width);
    let (sender2, receiver2) = signal::unsync::mutable(vec![width]);
    let (sender3, receiver3) = signal::unsync::mutable(vec![width]);
    let (text_sender, text_receiver) = signal::unsync::mutable(format!("{}", width));

    /*let style_width = receiver1.switch(move |x| {
        receiver2.clone().switch(move |y| {
            receiver3.clone().map(move |z| {
                Some(format!("{}px", x + y[0] + z[0]))
            })
        })
    });*/

    let style_width = map_rc! {
        let x: Rc<u32> = receiver1,
        let y: Rc<Vec<u32>> = receiver2,
        let z: Rc<Vec<u32>> = receiver3 =>
        Some(format!("{}px", *x + y[0] + z[0]))
    };


    dominator::append_dom(&document().query_selector("body").unwrap().unwrap(),
        html!("div", {
            style("border", "10px solid blue");
            children(&mut [
                text("Testing testing!!!"),

                text(text_receiver.dynamic()),

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
                        sender_elements.set(count).unwrap();
                    });
                    children(receiver_elements.map(|count| {
                        (0..count).map(|_| {
                            html!("div", {
                                style("border", "5px solid red");
                                style("width", "50px");
                                style("height", "50px");
                            })
                        })
                    }).dynamic());
                }),

                html!("div", {
                    style("width", "50px");
                    style("height", "50px");
                    style("background-color", "red");
                    children(&mut [
                        html!("div", {
                            style("width", "10px");
                            style("height", "10px");
                            style("background-color", "orange");
                        })
                    ]);
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
