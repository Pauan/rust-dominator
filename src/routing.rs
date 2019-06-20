use wasm_bindgen::{JsValue, UnwrapThrowExt};
use web_sys::{window, Url, EventTarget, HtmlElement};
use futures_signals::signal::{Mutable, Signal};
use gloo::events::EventListener;

use crate::dom::{Dom, DomBuilder};
use crate::events;


/*pub struct State<A> {
    value: Mutable<Option<A>>,
    callback: Value,
}

impl<A> State<A> {
    pub fn new() -> Self {
        // TODO replace with stdweb function
        let value = Mutable::new(js!( return history.state; ).try_into().unwrap_throw());

        let callback = |state: Option<A>| {
            value.set(state);
        };

        Self {
            value,
            callback: js!(
                var callback = @{callback};

                addEventListener("popstate", function (e) {
                    callback(e.state);
                }, true);

                return callback;
            ),
        }
    }

    pub fn set(&self, value: A) {
        window().history().replace_state(value, "", None).unwrap_throw();
        self.value.set(value);
    }
}*/


fn current_url_string() -> Result<String, JsValue> {
    Ok(window().unwrap_throw().location().href()?)
}

// TODO inline ?
fn change_url(mutable: &Mutable<Url>) -> Result<(), JsValue> {
    let mut lock = mutable.lock_mut();

    let new_url = current_url_string()?;

    // TODO test that this doesn't notify if the URLs are the same
    // TODO helper method for this
    // TODO can this be made more efficient ?
    if lock.href() != new_url {
        *lock = Url::new(&new_url)?;
    }

    Ok(())
}


struct CurrentUrl {
    value: Mutable<Url>,
    _listener: EventListener,
}

impl CurrentUrl {
    fn new() -> Result<Self, JsValue> {
        // TODO can this be made more efficient ?
        let value = Mutable::new(Url::new(&current_url_string()?)?);

        Ok(Self {
            _listener: EventListener::new(&window().unwrap_throw(), "popstate", {
                let value = value.clone();
                move |_| {
                    change_url(&value).unwrap_throw();
                }
            }),
            value,
        })
    }
}

// TODO somehow share this safely between threads ?
thread_local! {
    static URL: CurrentUrl = CurrentUrl::new().unwrap_throw();
}


#[inline]
pub fn current_url() -> Url {
    URL.with(|url| url.value.get_cloned())
}


#[inline]
pub fn url() -> impl Signal<Item = Url> {
    URL.with(|url| url.value.signal_cloned())
}


// TODO if URL hasn't been created yet, don't create it
#[inline]
pub fn go_to_url(new_url: &str) {
    window()
        .unwrap_throw()
        .history()
        .unwrap_throw()
        // TODO is this the best state object to use ?
        .push_state_with_url(&JsValue::NULL, "", Some(new_url))
        .unwrap_throw();

    URL.with(|url| {
        change_url(&url.value).unwrap_throw();
    });
}


// TODO somehow use &str rather than String, maybe Cow ?
#[inline]
pub fn on_click_go_to_url<A>(new_url: String) -> impl FnOnce(DomBuilder<A>) -> DomBuilder<A> where A: AsRef<EventTarget> {
    #[inline]
    move |dom| {
        dom.event_preventable(move |e: events::Click| {
            e.prevent_default();
            go_to_url(&new_url);
        })
    }
}


// TODO better type than HtmlElement
#[inline]
pub fn link<F>(url: &str, f: F) -> Dom where F: FnOnce(DomBuilder<HtmlElement>) -> DomBuilder<HtmlElement> {
    html!("a", {
        .attribute("href", url)
        // TODO somehow avoid this allocation
        .apply(on_click_go_to_url(url.to_string()))
        .apply(f)
    })
}
