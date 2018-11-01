use dom::{Dom, DomBuilder, Url};
use futures_signals::signal::{Mutable, Signal};
use stdweb::web::{window, HtmlElement, IEventTarget};
use stdweb::web::event::ClickEvent;
use stdweb::traits::IEvent;
use stdweb::Value;


/*pub struct State<A> {
    value: Mutable<Option<A>>,
    callback: Value,
}

impl<A> State<A> {
    pub fn new() -> Self {
        // TODO replace with stdweb function
        let value = Mutable::new(js!( return history.state; ).try_into().unwrap());

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
        window().history().replace_state(value, "", None).unwrap();
        self.value.set(value);
    }
}*/


fn current_url() -> String {
    window().location().unwrap().href().unwrap()
}

// TODO inline ?
fn change_url(mutable: &Mutable<Url>) {
    let new_url = current_url();

    let mut lock = mutable.lock_mut();

    // TODO test that this doesn't notify if the URLs are the same
    // TODO helper method for this
    // TODO can this be made more efficient ?
    if lock.href() != new_url {
        *lock = Url::new(&new_url);
    }
}


struct CurrentUrl {
    value: Mutable<Url>,
    listener: Value,
}

impl CurrentUrl {
    #[inline]
    fn new() -> Self {
        let value = Mutable::new(Url::new(&current_url()));

        let callback = {
            let value = value.clone();
            move || {
                change_url(&value);
            }
        };

        Self {
            value,
            listener: js!(
                function listener(e) {
                    @{callback}();
                }

                addEventListener("popstate", listener, true);

                return listener;
            ),
        }
    }
}

impl Drop for CurrentUrl {
    fn drop(&mut self) {
        js! { @(no_return)
            removeEventListener("popstate", @{&self.listener}, true);
        }
    }
}

// TODO use thread_local instead ?
lazy_static! {
    static ref URL: CurrentUrl = CurrentUrl::new();
}


#[inline]
pub fn url() -> impl Signal<Item = Url> {
    URL.value.signal_cloned()
}


// TODO if URL hasn't been created yet, don't create it
#[inline]
pub fn go_to_url(new_url: &str) {
    // TODO replace with stdweb function
    js! { @(no_return)
        // TODO is this the best state object to use ?
        history.pushState(null, "", @{new_url});
    }

    change_url(&URL.value);
}


// TODO somehow use &str rather than String
#[inline]
pub fn on_click_go_to_url<A>(new_url: String) -> impl FnOnce(DomBuilder<A>) -> DomBuilder<A> where A: IEventTarget + Clone + 'static {
    #[inline]
    move |dom| {
        dom.event(move |e: ClickEvent| {
            e.prevent_default();
            go_to_url(&new_url);
        })
    }
}


#[inline]
pub fn link<F>(url: &str, f: F) -> Dom where F: FnOnce(DomBuilder<HtmlElement>) -> DomBuilder<HtmlElement> {
    html!("a", {
        .attribute("href", url)
        // TODO somehow avoid this allocation
        .apply(on_click_go_to_url(url.to_string()))
        .apply(f)
    })
}
