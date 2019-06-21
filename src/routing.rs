use std::borrow::Cow;

use js_sys::JsString;
use web_sys::{EventTarget, HtmlElement};
use lazy_static::lazy_static;
use futures_signals::signal::{Mutable, ReadOnlyMutable};

use crate::bindings;
use crate::dom::{Dom, DomBuilder};
use crate::utils::EventListener;
use crate::events;


// TODO inline ?
fn change_url(mutable: &Mutable<String>) {
    let mut lock = mutable.lock_mut();

    let new_url = String::from(bindings::current_url());

    // TODO helper method for this
    // TODO can this be made more efficient ?
    if *lock != new_url {
        *lock = new_url;
    }
}


struct CurrentUrl {
    value: Mutable<String>,
}

impl CurrentUrl {
    fn new() -> Self {
        // TODO can this be made more efficient ?
        let value = Mutable::new(String::from(bindings::current_url()));

        // TODO clean this up somehow ?
        let _ = EventListener::new(bindings::window().into(), "popstate", {
            let value = value.clone();
            move |_| {
                change_url(&value);
            }
        });

        Self {
            value,
        }
    }
}


lazy_static! {
    static ref URL: CurrentUrl = CurrentUrl::new();
}


#[inline]
pub fn url() -> ReadOnlyMutable<String> {
    URL.value.read_only()
}


// TODO if URL hasn't been created yet, don't create it
#[inline]
pub fn go_to_url(new_url: &str) {
    // TODO intern ?
    bindings::go_to_url(&JsString::from(new_url));

    change_url(&URL.value);
}


#[inline]
pub fn on_click_go_to_url<A, B>(new_url: A) -> impl FnOnce(DomBuilder<B>) -> DomBuilder<B>
    where A: Into<Cow<'static, str>>,
          B: AsRef<EventTarget> {
    let new_url = new_url.into();

    #[inline]
    move |dom| {
        dom.event_preventable(move |e: events::Click| {
            e.prevent_default();
            go_to_url(&new_url);
        })
    }
}


// TODO better type than HtmlElement
// TODO maybe make this a macro ?
#[inline]
pub fn link<A, F>(url: A, f: F) -> Dom
    where A: Into<Cow<'static, str>>,
          F: FnOnce(DomBuilder<HtmlElement>) -> DomBuilder<HtmlElement> {
    let url = url.into();

    html!("a", {
        .attribute("href", &url)
        .apply(on_click_go_to_url(url))
        .apply(f)
    })
}
