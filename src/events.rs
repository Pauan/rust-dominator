use crate::traits::StaticEvent;
use wasm_bindgen::JsCast;
use web_sys::{EventTarget, HtmlInputElement, HtmlTextAreaElement};


macro_rules! make_event {
    ($name:ident, $type:literal => $event:path) => {
        pub struct $name {
            event: $event,
        }

        impl StaticEvent for $name {
            const EVENT_TYPE: &'static str = $type;

            #[inline]
            fn unchecked_from_event(event: web_sys::Event) -> Self {
                Self {
                    event: event.unchecked_into(),
                }
            }
        }

        impl $name {
            #[inline] pub fn prevent_default(&self) { self.event.prevent_default(); }

            #[inline] pub fn target(&self) -> Option<EventTarget> { self.event.target() }

            #[inline]
            pub fn dyn_target<A>(&self) -> Option<A> where A: JsCast {
                self.target()?.dyn_into().ok()
            }
        }
    };
}

macro_rules! make_mouse_event {
    ($name:ident, $type:literal) => {
        make_event!($name, $type => web_sys::MouseEvent);

        impl $name {
            #[inline] pub fn x(&self) -> i32 { self.event.client_x() }
            #[inline] pub fn y(&self) -> i32 { self.event.client_y() }

            #[inline] pub fn screen_x(&self) -> i32 { self.event.screen_x() }
            #[inline] pub fn screen_y(&self) -> i32 { self.event.screen_y() }
        }
    };
}

macro_rules! make_keyboard_event {
    ($name:ident, $type:literal) => {
        make_event!($name, $type => web_sys::KeyboardEvent);

        impl $name {
            // TODO return enum or something
            #[inline] pub fn key(&self) -> String { self.event.key() }
        }
    };
}

macro_rules! make_focus_event {
    ($name:ident, $type:literal) => {
        make_event!($name, $type => web_sys::FocusEvent);
    };
}


make_mouse_event!(Click, "click");
make_mouse_event!(MouseEnter, "mouseenter");
make_mouse_event!(MouseLeave, "mouseleave");
make_mouse_event!(DoubleClick, "dblclick");
make_mouse_event!(ContextMenu, "contextmenu");

make_keyboard_event!(KeyDown, "keydown");
make_keyboard_event!(KeyUp, "keyup");

make_focus_event!(Focus, "focus");
make_focus_event!(Blur, "blur");


make_event!(Input, "input" => web_sys::InputEvent);

impl Input {
    // TODO should this work on other types as well ?
    // TODO return JsString ?
    pub fn value(&self) -> Option<String> {
        let target = self.target()?;

        if let Some(target) = target.dyn_ref::<HtmlInputElement>() {
            // TODO check the <input> element's type ?
            Some(target.value())

        } else if let Some(target) = target.dyn_ref::<HtmlTextAreaElement>() {
            Some(target.value())

        } else {
            None
        }
    }
}


make_event!(Change, "change" => web_sys::Event);

// TODO add in a value method as well, the same as Input::value
impl Change {
    // https://developer.mozilla.org/en-US/docs/Web/API/HTMLInputElement
    pub fn checked(&self) -> Option<bool> {
        let target = self.dyn_target::<HtmlInputElement>()?;

        match target.type_().as_str() {
            "checkbox" | "radio" => Some(target.checked()),
            _ => None,
        }
    }
}
