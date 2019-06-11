use std::mem::ManuallyDrop;
use discard::Discard;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::{window, Document, EventTarget};
use gloo::events::{EventListener, EventListenerOptions};
use crate::traits::StaticEvent;


pub(crate) fn on<E, F>(element: &EventTarget, mut callback: F) -> EventListener
    where E: StaticEvent,
          F: FnMut(E) + 'static {
    EventListener::new(element, E::EVENT_TYPE, move |e| {
        callback(E::unchecked_from_event(e.clone()));
    })
}

pub(crate) fn on_with_options<E, F>(element: &EventTarget, options: EventListenerOptions, mut callback: F) -> EventListener
    where E: StaticEvent,
          F: FnMut(E) + 'static {
    EventListener::new_with_options(element, E::EVENT_TYPE, options, move |e| {
        callback(E::unchecked_from_event(e.clone()));
    })
}


pub(crate) fn document() -> Document {
    window().unwrap_throw().document().unwrap_throw()
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


// TODO is this worth using ?
pub(crate) struct EventDiscard(Option<EventListener>);

impl EventDiscard {
    #[inline]
    pub(crate) fn new(listener: EventListener) -> Self {
        EventDiscard(Some(listener))
    }
}

impl Drop for EventDiscard {
    #[inline]
    fn drop(&mut self) {
        // TODO does this cleanup as much memory as possible ?
        if let Some(listener) = self.0.take() {
            listener.forget();
        }
    }
}

impl Discard for EventDiscard {
    #[inline]
    fn discard(mut self) {
        self.0.take();
    }
}
