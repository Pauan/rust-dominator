#[macro_use]
extern crate stdweb;

#[macro_use]
extern crate dominator;

use std::rc::Rc;
use stdweb::web::{document, HtmlElement};
use stdweb::web::event::ClickEvent;
use stdweb::web::IParentNode;

use dominator::{Dom, signal};
use dominator::signal::Signal;


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

    let mut width = 100;

    let (sender, receiver) = signal::unsync::mutable(width);

    html!("div", {
        style("border", "10px solid blue");
        children(&mut [
            html!("div", {
                style("width", receiver.map(|x| Some(format!("{}px", x))));
                style("height", "50px");
                style("background-color", "green");
                event(move |event: ClickEvent| {
                    width += 100;
                    console!(log, &event);
                    sender.set(width).unwrap();
                });
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
    }).insert_into(
        &document().query_selector("body").unwrap().unwrap()
    );
}
