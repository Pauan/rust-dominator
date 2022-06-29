use std::mem::ManuallyDrop;

use wasm_bindgen::{JsCast, UnwrapThrowExt, intern};
use wasm_bindgen::closure::Closure;
use discard::Discard;
use web_sys::{EventTarget, Event, AddEventListenerOptions};

use crate::dom::EventOptions;
use crate::traits::StaticEvent;


// TODO should use gloo::events, but it doesn't support interning or Discard
pub(crate) struct EventListener {
    elem: EventTarget,
    name: &'static str,
    capture: bool,
    closure: Option<Closure<dyn FnMut(&Event)>>,
}

// TODO should these inline ?
impl EventListener {
    #[inline]
    pub(crate) fn new<F>(elem: EventTarget, name: &'static str, options: &EventOptions, callback: F) -> Self where F: FnMut(&Event) + 'static {
        let closure = Closure::wrap(Box::new(callback) as Box<dyn FnMut(&Event)>);
        let name: &'static str = intern(name);

        let capture = !options.bubbles;

        elem.add_event_listener_with_callback_and_add_event_listener_options(
            name,
            closure.as_ref().unchecked_ref(),
            AddEventListenerOptions::new()
                .capture(capture)
                .passive(!options.preventable),
        ).unwrap_throw();

        Self { elem, name, capture, closure: Some(closure) }
    }

    #[inline]
    pub(crate) fn once<F>(elem: EventTarget, name: &'static str, callback: F) -> Self where F: FnOnce(&Event) + 'static {
        let closure = Closure::once(callback);
        let name: &'static str = intern(name);

        elem.add_event_listener_with_callback_and_add_event_listener_options(
            name,
            closure.as_ref().unchecked_ref(),
            AddEventListenerOptions::new()
                .capture(true)
                .passive(true)
                .once(true),
        ).unwrap_throw();

        Self { elem, name, capture: true, closure: Some(closure) }
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

        self.elem.remove_event_listener_with_callback_and_bool(
            &self.name,
            closure.as_ref().unchecked_ref(),
            self.capture,
        ).unwrap_throw();
    }
}


#[inline]
pub(crate) fn on<E, F>(element: EventTarget, options: &EventOptions, mut callback: F) -> EventListener
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
