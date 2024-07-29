use std::borrow::Cow;
use std::cell::{Cell, RefCell, Ref, RefMut};
use std::mem::ManuallyDrop;

use wasm_bindgen::{JsValue, UnwrapThrowExt, intern};
use discard::{Discard, DiscardOnDrop};
use web_sys::{EventTarget, Event};
use futures_signals::signal::Mutable;

use crate::dom::EventOptions;
use crate::traits::StaticEvent;


pub(crate) struct RefCounter<A> {
    // Holds the data which is kept alive as long as `counter` is greater than 0
    data: RefCell<Option<A>>,

    // Used for ref-counting
    counter: Cell<usize>,
}

impl<A> RefCounter<A> {
    pub(crate) fn new() -> Self {
        Self {
            data: RefCell::new(None),
            counter: Cell::new(0),
        }
    }

    /// Gives a reference to the data.
    ///
    /// It will be `None` if the data hasn't been initialized yet.
    pub(crate) fn try_borrow(&self) -> Ref<'_, Option<A>> {
        self.data.borrow()
    }

    /// Decrements the ref count, cleaning up the data if the count is 0
    pub(crate) fn decrement(&self) {
        let counter = self.counter.get().checked_sub(1).unwrap();

        self.counter.set(counter);

        // No more listeners
        if counter == 0 {
            let _ = self.data.replace(None);
        }
    }

    /// Initializes the [`RefCounter`] and increments the ref count.
    ///
    /// If the [`RefCounter`] hasn't been initialized, it is initialized with the `FnOnce` closure.
    ///
    /// Regardless of whether it is initialized, it increments the ref count.
    pub(crate) fn increment<F>(&self, f: F) -> RefMut<'_, A> where F: FnOnce() -> A {
        let mut lock = self.data.borrow_mut();

        if lock.is_none() {
            *lock = Some(f());
        }

        let counter = self.counter.get().checked_add(1).unwrap();
        self.counter.set(counter);

        RefMut::map(lock, |data| data.as_mut().unwrap())
    }
}


pub(crate) struct MutableListener<A> {
    mutable: Mutable<A>,
    listener: DiscardOnDrop<EventListener>,
}

impl<A> MutableListener<A> {
    pub(crate) fn new(mutable: Mutable<A>, listener: EventListener) -> Self {
        Self {
            mutable,
            listener: DiscardOnDrop::new(listener),
        }
    }

    pub(crate) fn as_mutable(&self) -> &Mutable<A> {
        &self.mutable
    }
}


#[derive(Debug)]
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
