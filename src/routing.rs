use std::borrow::Cow;

use web_sys::{EventTarget, HtmlElement};
use futures_signals::signal::{Mutable, ReadOnlyMutable};

use crate::bindings;
use crate::bindings::WINDOW;
use crate::dom::{Dom, DomBuilder, EventOptions};
use crate::utils::{EventListener, MutableListener};
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


thread_local! {
    // TODO can this be made more efficient ?
    static CURRENT_URL: MutableListener<String> = MutableListener::new(String::from(bindings::current_url()));
}

fn with_url<A, F>(f: F) -> A where F: FnOnce(&Mutable<String>) -> A {
    CURRENT_URL.with(|url| {
        // TODO this needs to call decrement to clean up the listener
        url.increment(|url| {
            let url = url.clone();

            WINDOW.with(move |window| {
                EventListener::new(window, "popstate", &EventOptions::default(), move |_| {
                    change_url(&url);
                })
            })
        });

        f(url.as_mutable())
    })
}


pub fn url() -> ReadOnlyMutable<String> {
    with_url(|mutable| mutable.read_only())
}


/// Update the current route by adding a new entry to the history.
#[inline]
#[track_caller]
pub fn go_to_url(new_url: &str) {
    // TODO intern ?
    bindings::go_to_url(new_url);

    with_url(change_url);
}

/// Update the current route by replacing the history.
/// Use this very sparingly as this break the back button.
///
/// To let the user go back to the current route, use [`go_to_url`] instead.
#[inline]
#[track_caller]
pub fn replace_url(new_url: &str) {
    // TODO intern ?
    bindings::replace_url(new_url);

    with_url(change_url);
}

#[deprecated(since = "0.5.1", note = "Use the on_click_go_to_url macro instead")]
#[inline]
pub fn on_click_go_to_url<A, B>(new_url: A) -> impl FnOnce(DomBuilder<B>) -> DomBuilder<B>
    where A: Into<Cow<'static, str>>,
          B: AsRef<EventTarget> {
    let new_url = new_url.into();

    #[inline]
    move |dom| {
        dom.event_with_options(&EventOptions::preventable(), move |e: events::Click| {
            e.prevent_default();
            go_to_url(&new_url);
        })
    }
}


// TODO better type than HtmlElement
// TODO maybe make this a macro ?
#[deprecated(since = "0.5.1", note = "Use the link macro instead")]
#[allow(deprecated)]
#[inline]
pub fn link<A, F>(url: A, f: F) -> Dom
    where A: Into<Cow<'static, str>>,
          F: FnOnce(DomBuilder<HtmlElement>) -> DomBuilder<HtmlElement> {
    let url = url.into();

    html!("a", {
        .attr("href", &url)
        .apply(on_click_go_to_url(url))
        .apply(f)
    })
}


// TODO test this
/// Changes an `<a>` element to work with routing.
///
/// Normally when the user clicks on an `<a>` element the browser will handle the URL
/// routing.
///
/// But if you are creating a [Single Page Application](https://developer.mozilla.org/en-US/docs/Glossary/SPA) (SPA)
/// then you want your app to always be in control of routing.
///
/// The `on_click_go_to_url!` macro disables the browser routing for the `<a>`, so your app remains in control of routing:
///
/// ```rust
/// html!("a", {
///     .on_click_go_to_url!("/my-url/foo")
/// })
/// ```
///
/// Also see the [`link!`](crate::link) macro.
#[macro_export]
macro_rules! on_click_go_to_url {
    ($this:ident, $url:expr) => {{
        let url = $url;

        $this.event_with_options(&$crate::EventOptions::preventable(), move |e: $crate::events::Click| {
            e.prevent_default();
            $crate::routing::go_to_url(&url);
        })
    }};
}


// TODO test this
/// Creates an `<a>` element which works with routing.
///
/// Normally when the user clicks on an `<a>` element the browser will handle the URL
/// routing.
///
/// But if you are creating a [Single Page Application](https://developer.mozilla.org/en-US/docs/Glossary/SPA) (SPA)
/// then you want your app to always be in control of routing.
///
/// The `link!` macro creates an `<a>` element which disables browser routing, so your app remains in control of routing:
///
/// ```rust
/// link!("/my-url/foo", {
///     .class(...)
///     .style(...)
/// })
/// ```
///
/// Also see the [`on_click_go_to_url!`] macro.
#[macro_export]
macro_rules! link {
    ($url:expr, { $($methods:tt)* }) => {{
        let url = $url;

        $crate::html!("a", {
            .attr("href", &url)
            .apply(move |dom| $crate::on_click_go_to_url!(dom, url))
            $($methods)*
        })
    }};
}
