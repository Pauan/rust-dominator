use std::borrow::Cow;

use web_sys::{EventTarget, HtmlElement};
use once_cell::sync::Lazy;
use futures_signals::signal::{Mutable, ReadOnlyMutable};

use crate::bindings;
use crate::bindings::WINDOW;
use crate::dom::{Dom, DomBuilder, EventOptions};
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
        let _ = WINDOW.with(|window| {
            EventListener::new(window, "popstate", &EventOptions::default(), {
                let value = value.clone();
                move |_| {
                    change_url(&value);
                }
            }, true)
        });

        Self {
            value,
        }
    }
}


static URL: Lazy<CurrentUrl> = Lazy::new(|| CurrentUrl::new());


#[inline]
pub fn url() -> ReadOnlyMutable<String> {
    URL.value.read_only()
}


/// Update the current route by adding a new entry to the history.
// TODO if URL hasn't been created yet, don't create it
#[inline]
#[track_caller]
pub fn go_to_url(new_url: &str) {
    // TODO intern ?
    bindings::go_to_url(new_url);

    change_url(&URL.value);
}

/// Update the current route by replacing the history.
/// Use this very sparingly as this break the back button.
///
/// To let the user go back to the current route, use [`go_to_url`] instead.
// TODO if URL hasn't been created yet, don't create it
#[inline]
#[track_caller]
pub fn replace_url(new_url: &str) {
    // TODO intern ?
    bindings::replace_url(new_url);

    change_url(&URL.value);
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
