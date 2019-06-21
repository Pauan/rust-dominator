use std::mem::ManuallyDrop;

use wasm_bindgen::{JsCast, UnwrapThrowExt};
use wasm_bindgen::closure::Closure;
use js_sys::JsString;
use discard::Discard;
use web_sys::{EventTarget, Event};

use crate::bindings;
use crate::cache::intern;
use crate::traits::StaticEvent;


// TODO should use gloo::events, but it doesn't support interning or Discard
pub(crate) struct EventListener {
    elem: EventTarget,
    name: JsString,
    closure: Option<Closure<dyn FnMut(&Event)>>,
}

// TODO should these inline ?
impl EventListener {
    #[inline]
    pub(crate) fn new<F>(elem: EventTarget, name: &str, callback: F) -> Self where F: FnMut(&Event) + 'static {
        let closure = Closure::wrap(Box::new(callback) as Box<dyn FnMut(&Event)>);
        let name = intern(name);

        bindings::add_event(&elem, &name, closure.as_ref().unchecked_ref());

        Self { elem, name, closure: Some(closure) }
    }

    #[inline]
    pub(crate) fn new_preventable<F>(elem: EventTarget, name: &str, callback: F) -> Self where F: FnMut(&Event) + 'static {
        let closure = Closure::wrap(Box::new(callback) as Box<dyn FnMut(&Event)>);
        let name = intern(name);

        bindings::add_event_preventable(&elem, &name, closure.as_ref().unchecked_ref());

        Self { elem, name, closure: Some(closure) }
    }

    #[inline]
    pub(crate) fn once<F>(elem: EventTarget, name: &str, callback: F) -> Self where F: FnOnce(&Event) + 'static {
        let closure = Closure::once(callback);
        let name = intern(name);

        bindings::add_event_once(&elem, &name, closure.as_ref().unchecked_ref());

        Self { elem, name, closure: Some(closure) }
    }
}

impl Drop for EventListener {
    #[inline]
    fn drop(&mut self) {
        if let Some(closure) = self.closure.take() {
            // TODO can this be made more optimal ?
            closure.forget();
        }
    }
}

impl Discard for EventListener {
    #[inline]
    fn discard(mut self) {
        let closure = self.closure.take().unwrap_throw();
        bindings::remove_event(&self.elem, &self.name, closure.as_ref().unchecked_ref());
    }
}


#[inline]
pub(crate) fn on<E, F>(element: EventTarget, mut callback: F) -> EventListener
    where E: StaticEvent,
          F: FnMut(E) + 'static {
    EventListener::new(element, E::EVENT_TYPE, move |e| {
        callback(E::unchecked_from_event(e.clone()));
    })
}

#[inline]
pub(crate) fn on_preventable<E, F>(element: EventTarget, mut callback: F) -> EventListener
    where E: StaticEvent,
          F: FnMut(E) + 'static {
    EventListener::new_preventable(element, E::EVENT_TYPE, move |e| {
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
