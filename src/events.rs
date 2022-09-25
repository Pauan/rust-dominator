use crate::traits::StaticEvent;
use crate::EventOptions;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{EventTarget, HtmlInputElement, HtmlTextAreaElement, TouchList, Touch};


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
            event: crate::__unwrap!(
                event.dyn_into(),
                e => panic!("Invalid event type: {:?}", wasm_bindgen::JsValue::as_ref(&e)),
            ),
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


macro_rules! static_event_impl {
    ($name:ident => $type:literal) => {
        impl StaticEvent for $name {
            const EVENT_TYPE: &'static str = $type;

            #[inline]
            fn unchecked_from_event(event: web_sys::Event) -> Self {
                Self {
                    event: event.unchecked_into(),
                }
            }
        }
    };
}

macro_rules! make_event {
    ($name:ident => $event:path) => {
        #[derive(Debug)]
        pub struct $name {
            event: $event,
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
    ($name:ident => $event:path) => {
        make_event!($name => $event);

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

macro_rules! make_pointer_event {
    ($name:ident) => {
        make_mouse_event!($name => web_sys::PointerEvent);

        impl $name {
            #[inline] pub fn pointer_id(&self) -> i32 { self.event.pointer_id() }

            #[inline] pub fn pointer_width(&self) -> i32 { self.event.width() }
            #[inline] pub fn pointer_height(&self) -> i32 { self.event.height() }

            #[inline] pub fn pressure(&self) -> f32 { self.event.pressure() }
            #[inline] pub fn tangential_pressure(&self) -> f32 { self.event.tangential_pressure() }

            #[inline] pub fn tilt_x(&self) -> i32 { self.event.tilt_x() }
            #[inline] pub fn tilt_y(&self) -> i32 { self.event.tilt_y() }

            #[inline] pub fn twist(&self) -> i32 { self.event.twist() }

            #[inline] pub fn is_primary(&self) -> bool { self.event.is_primary() }
        }
    };
}

macro_rules! make_touch_event {
    ($name:ident) => {
        make_event!($name => web_sys::TouchEvent);

        impl $name {
            #[inline] pub fn ctrl_key(&self) -> bool { self.event.ctrl_key() || self.event.meta_key() }
            #[inline] pub fn shift_key(&self) -> bool { self.event.shift_key() }
            #[inline] pub fn alt_key(&self) -> bool { self.event.alt_key() }

            #[inline]
            pub fn changed_touches(&self) -> impl Iterator<Item = Touch> {
                TouchListIter::new(self.event.changed_touches())
            }

            #[inline]
            pub fn target_touches(&self) -> impl Iterator<Item = Touch> {
                TouchListIter::new(self.event.target_touches())
            }

            #[inline]
            pub fn touches(&self) -> impl Iterator<Item = Touch> {
                TouchListIter::new(self.event.touches())
            }
        }
    };
}

macro_rules! make_keyboard_event {
    ($name:ident) => {
        make_event!($name => web_sys::KeyboardEvent);

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
    ($name:ident) => {
        make_event!($name => web_sys::FocusEvent);

        impl $name {
            #[inline] pub fn related_target(&self) -> Option<EventTarget> { self.event.related_target() }
        }
    };
}

macro_rules! make_drag_event {
    ($name:ident) => {
        make_mouse_event!($name => web_sys::DragEvent);

        impl $name {
            #[inline] pub fn data_transfer(&self) -> Option<web_sys::DataTransfer> { self.event.data_transfer() }
        }
    };
}

macro_rules! make_input_event {
    ($name:ident) => {
        make_event!($name => web_sys::InputEvent);

        impl $name {
            #[inline] pub fn data(&self) -> Option<String> { self.event.data() }
        }
    };
}

macro_rules! make_animation_event {
    ($name:ident) => {
        make_event!($name => web_sys::AnimationEvent);

        impl $name {
            #[inline] pub fn animation_name(&self) -> String { self.event.animation_name() }
            #[inline] pub fn elapsed_time(&self) -> f32 { self.event.elapsed_time() }
            #[inline] pub fn pseudo_element(&self) -> String { self.event.pseudo_element() }
        }
    };
}

macro_rules! make_wheel_event {
    ($name:ident) => {
        make_mouse_event!($name => web_sys::WheelEvent);

        impl $name {
            #[inline] pub fn delta_x(&self) -> f64 { self.event.delta_x() }
            #[inline] pub fn delta_y(&self) -> f64 { self.event.delta_y() }
            #[inline] pub fn delta_z(&self) -> f64 { self.event.delta_z() }
        }
    };
}

macro_rules! make_message_event {
    ($name:ident) => {
        make_event!($name => web_sys::MessageEvent);

        impl $name {
            #[inline] pub fn data(&self) -> JsValue { self.event.data() }
        }
    };
}

make_mouse_event!(Click => web_sys::MouseEvent);
static_event_impl!(Click => "click");

make_mouse_event!(MouseDown => web_sys::MouseEvent);
static_event_impl!(MouseDown => "mousedown");

make_mouse_event!(MouseUp => web_sys::MouseEvent);
static_event_impl!(MouseUp => "mouseup");

make_mouse_event!(MouseMove => web_sys::MouseEvent);
static_event_impl!(MouseMove => "mousemove");


make_mouse_event!(MouseEnter => web_sys::MouseEvent);
make_mouse_event!(MouseLeave => web_sys::MouseEvent);

impl StaticEvent for MouseEnter {
    const EVENT_TYPE: &'static str = "mouseenter";

    #[inline]
    fn unchecked_from_event(event: web_sys::Event) -> Self {
        Self {
            event: event.unchecked_into(),
        }
    }

    #[inline]
    fn default_options(preventable: bool) -> EventOptions {
        EventOptions {
            bubbles: true,
            preventable,
        }
    }
}

impl StaticEvent for MouseLeave {
    const EVENT_TYPE: &'static str = "mouseleave";

    #[inline]
    fn unchecked_from_event(event: web_sys::Event) -> Self {
        Self {
            event: event.unchecked_into(),
        }
    }

    #[inline]
    fn default_options(preventable: bool) -> EventOptions {
        EventOptions {
            bubbles: true,
            preventable,
        }
    }
}


make_mouse_event!(DoubleClick => web_sys::MouseEvent);
static_event_impl!(DoubleClick => "dblclick");

make_mouse_event!(ContextMenu => web_sys::MouseEvent);
static_event_impl!(ContextMenu => "contextmenu");

make_pointer_event!(PointerOver);
static_event_impl!(PointerOver => "pointerover");

make_pointer_event!(PointerEnter);
static_event_impl!(PointerEnter => "pointerenter");

make_pointer_event!(PointerDown);
static_event_impl!(PointerDown => "pointerdown");

make_pointer_event!(PointerMove);
static_event_impl!(PointerMove => "pointermove");

make_pointer_event!(PointerUp);
static_event_impl!(PointerUp => "pointerup");

make_pointer_event!(PointerCancel);
static_event_impl!(PointerCancel => "pointercancel");

make_pointer_event!(PointerOut);
static_event_impl!(PointerOut => "pointerout");

make_pointer_event!(PointerLeave);
static_event_impl!(PointerLeave => "pointerleave");

make_pointer_event!(GotPointerCapture);
static_event_impl!(GotPointerCapture => "gotpointercapture");

make_pointer_event!(LostPointerCapture);
static_event_impl!(LostPointerCapture => "lostpointercapture");

make_keyboard_event!(KeyDown);
static_event_impl!(KeyDown => "keydown");

make_keyboard_event!(KeyUp);
static_event_impl!(KeyUp => "keyup");


make_focus_event!(Focus);
static_event_impl!(Focus => "focus");

make_focus_event!(Blur);
static_event_impl!(Blur => "blur");

make_focus_event!(FocusIn);
static_event_impl!(FocusIn => "focusin");

make_focus_event!(FocusOut);
static_event_impl!(FocusOut => "focusout");


make_drag_event!(DragStart);
static_event_impl!(DragStart => "dragstart");

make_drag_event!(Drag);
static_event_impl!(Drag => "drag");

make_drag_event!(DragEnd);
static_event_impl!(DragEnd => "dragend");

make_drag_event!(DragOver);
static_event_impl!(DragOver => "dragover");

make_drag_event!(DragEnter);
static_event_impl!(DragEnter => "dragenter");

make_drag_event!(DragLeave);
static_event_impl!(DragLeave => "dragleave");

make_drag_event!(Drop);
static_event_impl!(Drop => "drop");


make_input_event!(Input);
static_event_impl!(Input => "input");

make_input_event!(BeforeInput);
static_event_impl!(BeforeInput => "beforeinput");


make_animation_event!(AnimationStart);
static_event_impl!(AnimationStart => "animationstart");

make_animation_event!(AnimationIteration);
static_event_impl!(AnimationIteration => "animationiteration");

make_animation_event!(AnimationCancel);
static_event_impl!(AnimationCancel => "animationcancel");

make_animation_event!(AnimationEnd);
static_event_impl!(AnimationEnd => "animationend");


make_wheel_event!(Wheel);
static_event_impl!(Wheel => "wheel");


make_message_event!(Message);
static_event_impl!(Message => "message");


make_event!(Load => web_sys::Event);
static_event_impl!(Load => "load");

make_event!(Error => web_sys::Event);
static_event_impl!(Error => "error");

make_event!(Scroll => web_sys::Event);
static_event_impl!(Scroll => "scroll");

make_event!(Submit => web_sys::Event);
static_event_impl!(Submit => "submit");

make_event!(Resize => web_sys::UiEvent);
static_event_impl!(Resize => "resize");

make_event!(SelectionChange => web_sys::Event);
static_event_impl!(SelectionChange => "selectionchange");



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


make_event!(Change => web_sys::Event);
static_event_impl!(Change => "change");

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


make_touch_event!(TouchCancel);
static_event_impl!(TouchCancel => "touchcancel");

make_touch_event!(TouchEnd);
static_event_impl!(TouchEnd => "touchend");

make_touch_event!(TouchMove);
static_event_impl!(TouchMove => "touchmove");

make_touch_event!(TouchStart);
static_event_impl!(TouchStart => "touchstart");


#[derive(Debug)]
struct TouchListIter {
    list: TouchList,
    index: u32,
    length: u32,
}

impl TouchListIter {
    fn new(list: TouchList) -> Self {
        Self {
            index: 0,
            length: list.length(),
            list,
        }
    }
}

impl Iterator for TouchListIter {
    type Item = Touch;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;

        if index < self.length {
            self.index += 1;
            self.list.get(index)
        } else {
            None
        }
    }
}
