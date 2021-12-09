use crate::traits::StaticEvent;
use wasm_bindgen::JsCast;
use web_sys::{EventTarget, HtmlInputElement, HtmlTextAreaElement};


#[cfg(feature = "nightly")]
pub struct Event<const NAME: &'static str, T> {
    event: T,
}

#[cfg(feature = "nightly")]
impl<T, const NAME: &'static str> StaticEvent for Event<NAME, T> where T: JsCast {
    const EVENT_TYPE: &'static str = NAME;

    #[inline]
    fn unchecked_from_event(event: web_sys::Event) -> Self {
        Self {
            // TODO use unchecked_into in release mode ?
            event: event.dyn_into().unwrap(),
        }
    }
}

// TODO code duplication
// TODO implement the rest of the methods
#[cfg(feature = "nightly")]
impl<T, const NAME: &'static str> Event<NAME, T> where T: AsRef<web_sys::Event> {
    #[inline] pub fn prevent_default(&self) { self.event.as_ref().prevent_default(); }

    #[inline] pub fn target(&self) -> Option<EventTarget> { self.event.as_ref().target() }

    #[inline]
    pub fn dyn_target<A>(&self) -> Option<A> where A: JsCast {
        self.target()?.dyn_into().ok()
    }
}


macro_rules! make_event {
    ($name:ident, $type:literal => $event:path) => {
        #[derive(Debug)]
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

            #[inline] pub fn stop_propagation(&self) { self.event.stop_propagation(); }

            #[inline] pub fn stop_immediate_propagation(&self) { self.event.stop_immediate_propagation(); }

            #[inline] pub fn target(&self) -> Option<EventTarget> { self.event.target() }

            #[inline]
            pub fn dyn_target<A>(&self) -> Option<A> where A: JsCast {
                self.target()?.dyn_into().ok()
            }
        }
    };
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    Button4,
    Button5,
}

macro_rules! make_mouse_event {
    ($name:ident, $type:literal => $event:path) => {
        make_event!($name, $type => $event);

        impl $name {
            #[inline] pub fn x(&self) -> i32 { self.event.client_x() }
            #[inline] pub fn y(&self) -> i32 { self.event.client_y() }

            #[inline] pub fn movement_x(&self) -> i32 { self.event.movement_x() }
            #[inline] pub fn movement_y(&self) -> i32 { self.event.movement_y() }

            #[inline] pub fn offset_x(&self) -> i32 { self.event.offset_x() }
            #[inline] pub fn offset_y(&self) -> i32 { self.event.offset_y() }

            #[inline] pub fn page_x(&self) -> i32 { self.event.page_x() }
            #[inline] pub fn page_y(&self) -> i32 { self.event.page_y() }

            #[inline] pub fn screen_x(&self) -> i32 { self.event.screen_x() }
            #[inline] pub fn screen_y(&self) -> i32 { self.event.screen_y() }

            #[inline] pub fn ctrl_key(&self) -> bool { self.event.ctrl_key() || self.event.meta_key() }
            #[inline] pub fn shift_key(&self) -> bool { self.event.shift_key() }
            #[inline] pub fn alt_key(&self) -> bool { self.event.alt_key() }

            // TODO maybe deprecate these ?
            #[inline] pub fn mouse_x(&self) -> i32 { self.event.client_x() }
            #[inline] pub fn mouse_y(&self) -> i32 { self.event.client_y() }

            pub fn button(&self) -> MouseButton {
                match self.event.button() {
                    0 => MouseButton::Left,
                    1 => MouseButton::Middle,
                    2 => MouseButton::Right,
                    3 => MouseButton::Button4,
                    4 => MouseButton::Button5,
                    _ => unreachable!("Unexpected MouseEvent.button value"),
                }
            }
        }
    };
}

macro_rules! make_keyboard_event {
    ($name:ident, $type:literal) => {
        make_event!($name, $type => web_sys::KeyboardEvent);

        impl $name {
            // TODO return enum or something
            #[inline] pub fn key(&self) -> String { self.event.key() }

            #[inline] pub fn ctrl_key(&self) -> bool { self.event.ctrl_key() || self.event.meta_key() }
            #[inline] pub fn shift_key(&self) -> bool { self.event.shift_key() }
            #[inline] pub fn alt_key(&self) -> bool { self.event.alt_key() }
            #[inline] pub fn repeat(&self) -> bool { self.event.repeat() }
        }
    };
}

macro_rules! make_focus_event {
    ($name:ident, $type:literal) => {
        make_event!($name, $type => web_sys::FocusEvent);
    };
}

macro_rules! make_drag_event {
    ($name:ident, $type:literal) => {
        make_mouse_event!($name, $type => web_sys::DragEvent);

        impl $name {
            #[inline] pub fn data_transfer(&self) -> Option<web_sys::DataTransfer> { self.event.data_transfer() }
        }
    };
}

macro_rules! make_input_event {
    ($name:ident, $type:literal) => {
        make_event!($name, $type => web_sys::InputEvent);

        impl $name {
            #[inline] pub fn data(&self) -> Option<String> { self.event.data() }
        }
    };
}


make_mouse_event!(Click, "click" => web_sys::MouseEvent);
make_mouse_event!(MouseDown, "mousedown" => web_sys::MouseEvent);
make_mouse_event!(MouseUp, "mouseup" => web_sys::MouseEvent);
make_mouse_event!(MouseMove, "mousemove" => web_sys::MouseEvent);
make_mouse_event!(MouseEnter, "mouseenter" => web_sys::MouseEvent);
make_mouse_event!(MouseLeave, "mouseleave" => web_sys::MouseEvent);
make_mouse_event!(DoubleClick, "dblclick" => web_sys::MouseEvent);
make_mouse_event!(ContextMenu, "contextmenu" => web_sys::MouseEvent);

make_keyboard_event!(KeyDown, "keydown");
make_keyboard_event!(KeyUp, "keyup");

make_focus_event!(Focus, "focus");
make_focus_event!(Blur, "blur");

make_drag_event!(DragStart, "dragstart");
make_drag_event!(Drag, "drag");
make_drag_event!(DragEnd, "dragend");
make_drag_event!(DragOver, "dragover");
make_drag_event!(DragEnter, "dragenter");
make_drag_event!(DragLeave, "dragleave");
make_drag_event!(Drop, "drop");

make_input_event!(Input, "input");
make_input_event!(BeforeInput, "beforeinput");

make_event!(Load, "load" => web_sys::Event);
make_event!(Scroll, "scroll" => web_sys::Event);
make_event!(Resize, "resize" => web_sys::UiEvent);


impl Input {
    // TODO should this work on other types as well ?
    #[deprecated(since = "0.5.19", note = "Use with_node instead")]
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
