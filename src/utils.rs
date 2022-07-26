use std::borrow::Cow;
use std::mem::ManuallyDrop;

use discard::Discard;
use wasm_bindgen::intern;
use web_sys::{Event, EventTarget};

use crate::dom::EventOptions;
use crate::traits::StaticEvent;

pub(crate) struct EventListener(Option<gloo_events::EventListener>);

// TODO should these inline ?
impl EventListener {
    #[inline]
    pub(crate) fn new<N, F>(
        elem: &EventTarget,
        name: N,
        options: &EventOptions,
        callback: F,
    ) -> Self
    where
        N: Into<Cow<'static, str>>,
        F: FnMut(&Event) + 'static,
    {
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
    where
        N: Into<Cow<'static, str>>,
        F: FnOnce(&Event) + 'static,
    {
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
            }
            .into_gloo(),
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
        let _ = self.0.take().unwrap();
    }
}

#[inline]
pub(crate) fn on<E, F>(
    element: &EventTarget,
    options: &EventOptions,
    mut callback: F,
) -> EventListener
where
    E: StaticEvent,
    F: FnMut(E) + 'static,
{
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

impl<A> FnDiscard<A>
where
    A: FnOnce(),
{
    #[inline]
    pub(crate) fn new(f: A) -> Self {
        FnDiscard(f)
    }
}

impl<A> Discard for FnDiscard<A>
where
    A: FnOnce(),
{
    #[inline]
    fn discard(self) {
        self.0();
    }
}
