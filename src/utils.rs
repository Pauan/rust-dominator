use std::borrow::Cow;
use std::mem::ManuallyDrop;

use wasm_bindgen::{JsValue, UnwrapThrowExt, intern};
use discard::Discard;
use web_sys::{EventTarget, Event};

use crate::dom::EventOptions;
use crate::traits::StaticEvent;


pub(crate) struct EventListener(Option<gloo_events::EventListener>);

// TODO should these inline ?
impl EventListener {
    #[inline]
    pub(crate) fn new<N, F>(elem: &EventTarget, name: N, options: &EventOptions, callback: F) -> Self
        where N: Into<Cow<'static, str>>,
              F: FnMut(&Event) + 'static {

        // TODO get rid of this by fixing web-sys code generation
        intern("capture");
        intern("once");
        intern("passive");

        let name = name.into();
        intern(&name);

        Self(Some(gloo_events::EventListener::new_with_options(
            elem,
            name,
            options.into_gloo(),
            callback,
        )))
    }

    #[inline]
    pub(crate) fn once<N, F>(elem: &EventTarget, name: N, callback: F) -> Self
        where N: Into<Cow<'static, str>>,
              F: FnOnce(&Event) + 'static {

        // TODO get rid of this by fixing web-sys code generation
        intern("capture");
        intern("once");
        intern("passive");

        let name = name.into();
        intern(&name);

        Self(Some(gloo_events::EventListener::once_with_options(
            elem,
            name,
            EventOptions {
                bubbles: false,
                preventable: false,
            }.into_gloo(),
            callback,
        )))
    }
}

impl Drop for EventListener {
    #[inline]
    fn drop(&mut self) {
        if let Some(listener) = self.0.take() {
            // TODO can this be made more optimal ?
            listener.forget();
        }
    }
}

impl Discard for EventListener {
    #[inline]
    fn discard(mut self) {
        // Drops the listener which cleans it up
        let _ = self.0.take().unwrap_throw();
    }
}


#[inline]
pub(crate) fn on<E, F>(element: &EventTarget, options: &EventOptions, mut callback: F) -> EventListener
    where E: StaticEvent,
          F: FnMut(E) + 'static {
    EventListener::new(element, E::EVENT_TYPE, options, move |e| {
        callback(E::unchecked_from_event(e.clone()));
    })
}


// TODO move this into the discard crate
// TODO verify that this is correct and doesn't leak memory or cause memory safety
pub(crate) struct ValueDiscard<A>(ManuallyDrop<A>);

impl<A> ValueDiscard<A> {
    #[inline]
    pub(crate) fn new(value: A) -> Self {
        ValueDiscard(ManuallyDrop::new(value))
    }
}

impl<A> Discard for ValueDiscard<A> {
    #[inline]
    fn discard(self) {
        // TODO verify that this works
        ManuallyDrop::into_inner(self.0);
    }
}


// TODO move this into the discard crate
// TODO replace this with an impl for FnOnce() ?
pub(crate) struct FnDiscard<A>(A);

impl<A> FnDiscard<A> where A: FnOnce() {
    #[inline]
    pub(crate) fn new(f: A) -> Self {
        FnDiscard(f)
    }
}

impl<A> Discard for FnDiscard<A> where A: FnOnce() {
    #[inline]
    fn discard(self) {
        self.0();
    }
}


pub(crate) trait UnwrapJsExt<T> {
    fn unwrap_js(self) -> T;
}

#[cfg(debug_assertions)]
impl<T> UnwrapJsExt<T> for Result<T, JsValue> {
    #[inline]
    #[track_caller]
    fn unwrap_js(self) -> T {
        match self {
            Ok(value) => value,
            Err(e) => {
                use wasm_bindgen::JsCast;

                match e.dyn_ref::<js_sys::Error>() {
                    Some(e) => {
                        panic!("{}", e.message());
                    },
                    // TODO test this
                    None => {
                        panic!("{:?}", e);
                    },
                }
            },
        }
    }
}

#[cfg(not(debug_assertions))]
impl<T> UnwrapJsExt<T> for Result<T, JsValue> {
    #[inline]
    fn unwrap_js(self) -> T {
        self.unwrap_or_else(|e| wasm_bindgen::throw_val(e))
    }
}


// This needs to be a macro because #[track_caller] isn't supported in closures
// https://github.com/rust-lang/rust/issues/87417
#[doc(hidden)]
#[macro_export]
macro_rules! __unwrap {
    ($value:expr, $var:ident => $error:expr,) => {{
        #[cfg(debug_assertions)]
        match $value {
            Ok(value) => value,
            Err($var) => $error,
        }

        #[cfg(not(debug_assertions))]
        $value.unwrap_throw()
    }};
}
