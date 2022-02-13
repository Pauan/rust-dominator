use std::borrow::Cow;

use futures_signals::signal::{Mutable, ReadOnlyMutable};
use once_cell::sync::Lazy;
use web_sys::{EventTarget, HtmlElement};

use crate::bindings;
use crate::dom::{Dom, DomBuilder, EventOptions};
use crate::events;
use crate::utils::EventListener;

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
        let _ = EventListener::new(
            bindings::window_event_target(),
            "popstate",
            &EventOptions::default(),
            {
                let value = value.clone();
                move |_| {
                    change_url(&value);
                }
            },
        );

        Self { value }
    }
}

static URL: Lazy<CurrentUrl> = Lazy::new(|| CurrentUrl::new());

/// Get the current URL (read-only) and the ability to register signals for when it updates.
#[inline]
pub fn url() -> ReadOnlyMutable<String> {
    URL.value.read_only()
}

/// Navigate to the given URL.
// TODO if URL hasn't been created yet, don't create it
#[inline]
pub fn go_to_url(new_url: &str) {
    // TODO intern ?
    bindings::go_to_url(new_url);

    change_url(&URL.value);
}

/// Makes an element navigate to the given url when clicked.
#[deprecated(since = "0.5.1", note = "Use the on_click_go_to_url macro instead")]
#[inline]
pub fn on_click_go_to_url<A, B>(new_url: A) -> impl FnOnce(DomBuilder<B>) -> DomBuilder<B>
where
    A: Into<Cow<'static, str>>,
    B: AsRef<EventTarget>,
{
    let new_url = new_url.into();

    #[inline]
    move |dom| {
        dom.event_with_options(&EventOptions::preventable(), move |e: events::Click| {
            e.prevent_default();
            go_to_url(&new_url);
        })
    }
}

/// Wrap `f` in an `<a>` link with given URL.
// TODO better type than HtmlElement
// TODO maybe make this a macro ?
#[deprecated(since = "0.5.1", note = "Use the link macro instead")]
#[allow(deprecated)]
#[inline]
pub fn link<A, F>(url: A, f: F) -> Dom
where
    A: Into<Cow<'static, str>>,
    F: FnOnce(DomBuilder<HtmlElement>) -> DomBuilder<HtmlElement>,
{
    let url = url.into();

    html!("a", {
        .attribute("href", &url)
        .apply(on_click_go_to_url(url))
        .apply(f)
    })
}

/// Attaches an event handler to the given object that navigates to `$url` when clicked.
///
/// # Examples
///
/// ```no_run
/// # use dominator::{html, on_click_go_to_url};
/// html!("div", {
///     // here `apply` gives us access to the `web_sys::Element`.
///     .apply(move |dom| on_click_go_to_url!(dom, "/clicked"))
///     .text("click me")
/// });
/// ```
// TODO test this
#[macro_export]
macro_rules! on_click_go_to_url {
    ($this:ident, $url:expr) => {{
        let url = $url;

        $this.event_with_options(
            &$crate::EventOptions::preventable(),
            move |e: $crate::events::Click| {
                e.prevent_default();
                $crate::routing::go_to_url(&url);
            },
        )
    }};
}

/// Crate a `<a>` link that points to `$url`.
///
/// Actually handles the navigation directly rather than forwarding to the browser, so that we can
/// control what happens. Equivalent to
///
/// ```no_run
/// # use dominator::{html, on_click_go_to_url};
/// let url = "/my/url.html";
/// html!("a", {
///     .attribute("href", &url)
///     .apply(move |dom| on_click_go_to_url!(dom, url))
///     // your `$methods` go here
/// });
/// ```
///
/// # Examples
///
/// ```no_run
/// # use dominator::{html, link};
/// let dom = html!("div", {
///     .children(&mut [
///         link!("/my_page.html", {
///             .attribute("name", "my_link")
///         })
///     ])
/// });
/// ```
// TODO test this
#[macro_export]
macro_rules! link {
    ($url:expr, { $($methods:tt)* }) => {{
        let url = $url;

        $crate::html!("a", {
            .attribute("href", &url)
            .apply(move |dom| $crate::on_click_go_to_url!(dom, url))
            $($methods)*
        })
    }};
}
