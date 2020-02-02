use std::pin::Pin;
use std::convert::AsRef;
use std::future::Future;
use std::task::{Context, Poll};

use lazy_static::lazy_static;
use futures_signals::signal::{Signal, not};
use futures_signals::signal_vec::SignalVec;
use futures_util::FutureExt;
use futures_channel::oneshot;
use discard::{Discard, DiscardOnDrop};
use wasm_bindgen::{JsValue, UnwrapThrowExt, JsCast, intern};
use web_sys::{HtmlElement, Node, EventTarget, Element, CssStyleSheet, CssStyleDeclaration};

use crate::bindings;
use crate::callbacks::Callbacks;
use crate::traits::*;
use crate::operations;
use crate::operations::{for_each, spawn_future};
use crate::utils::{EventListener, on, on_preventable, ValueDiscard, FnDiscard};


pub struct RefFn<A, B, C> where B: ?Sized, C: Fn(&A) -> &B {
    value: A,
    callback: C,
}

impl<A, B, C> RefFn<A, B, C> where B: ?Sized, C: Fn(&A) -> &B {
    #[inline]
    pub fn new(value: A, callback: C) -> Self {
        Self {
            value,
            callback,
        }
    }

    #[inline]
    pub fn call_ref(&self) -> &B {
        (self.callback)(&self.value)
    }

    /*pub fn map<D, E>(self, callback: E) -> RefFn<A, impl Fn(&A) -> &D>
        where D: ?Sized,
              E: Fn(&B) -> &D {

        let old_callback = self.callback;

        RefFn {
            value: self.value,
            callback: move |value| callback(old_callback(value)),
        }
    }*/
}

/*impl<A, B, C> Deref for RefFn<A, C> where B: ?Sized, C: Fn(&A) -> &B {
    type Target = B;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.call_ref()
    }
}*/

/*impl<A, B, C> AsRef<B> for RefFn<A, C> where B: ?Sized, C: Fn(&A) -> &B {
    #[inline]
    fn as_ref(&self) -> &B {
        self.call_ref()
    }
}*/


// https://developer.mozilla.org/en-US/docs/Web/API/Document/createElementNS#Valid%20Namespace%20URIs
const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

// 32-bit signed int
pub const HIGHEST_ZINDEX: &str = "2147483647";


lazy_static! {
    static ref HIDDEN_CLASS: String = class! {
        .style_important("display", "none")
    };
}


// TODO should return HtmlBodyElement ?
pub fn body() -> HtmlElement {
    bindings::body()
}


pub fn get_id(id: &str) -> Element {
    // TODO intern ?
    bindings::get_element_by_id(id)
}


pub struct DomHandle {
    parent: Node,
    dom: Dom,
}

impl Discard for DomHandle {
    #[inline]
    fn discard(self) {
        bindings::remove_child(&self.parent, &self.dom.element);
        self.dom.callbacks.discard();
    }
}

#[inline]
pub fn append_dom(parent: &Node, mut dom: Dom) -> DomHandle {
    bindings::append_child(&parent, &dom.element);

    dom.callbacks.trigger_after_insert();

    // This prevents it from triggering after_remove
    dom.callbacks.leak();

    DomHandle {
        parent: parent.clone(),
        dom,
    }
}


// TODO use must_use ?
enum IsWindowLoaded {
    Initial {},
    Pending {
        receiver: oneshot::Receiver<Option<bool>>,
        _event: EventListener,
    },
    Done {},
}

impl Signal for IsWindowLoaded {
    type Item = bool;

    fn poll_change(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let result = match *self {
            IsWindowLoaded::Initial {} => {
                let is_ready = bindings::ready_state() == "complete";

                if is_ready {
                    Poll::Ready(Some(true))

                } else {
                    let (sender, receiver) = oneshot::channel();

                    *self = IsWindowLoaded::Pending {
                        receiver,
                        _event: EventListener::once(bindings::window_event_target(), "load", move |_| {
                            // TODO test this
                            sender.send(Some(true)).unwrap_throw();
                        }),
                    };

                    Poll::Ready(Some(false))
                }
            },
            IsWindowLoaded::Pending { ref mut receiver, .. } => {
                receiver.poll_unpin(cx).map(|x| x.unwrap_throw())
            },
            IsWindowLoaded::Done {} => {
                Poll::Ready(None)
            },
        };

        if let Poll::Ready(Some(true)) = result {
            *self = IsWindowLoaded::Done {};
        }

        result
    }
}

// TODO this should be moved into gloo
#[inline]
pub fn is_window_loaded() -> impl Signal<Item = bool> {
    IsWindowLoaded::Initial {}
}


// TODO should this intern ?
#[inline]
pub fn text(value: &str) -> Dom {
    Dom::new(bindings::create_text_node(value).into())
}


// TODO should this inline ?
pub fn text_signal<A, B>(value: B) -> Dom
    where A: AsStr,
          B: Signal<Item = A> + 'static {

    let element = bindings::create_text_node(intern(""));

    let mut callbacks = Callbacks::new();

    {
        let element = element.clone();

        callbacks.after_remove(for_each(value, move |value| {
            let value = value.as_str();

            // TODO maybe this should intern ?
            bindings::set_text(&element, value);
        }));
    }

    Dom {
        element: element.into(),
        callbacks: callbacks,
    }
}


// TODO better warning message for must_use
#[must_use]
#[derive(Debug)]
pub struct Dom {
    pub(crate) element: Node,
    pub(crate) callbacks: Callbacks,
}

impl Dom {
    #[inline]
    pub fn new(element: Node) -> Self {
        Self {
            element,
            callbacks: Callbacks::new(),
        }
    }

    #[inline]
    pub fn empty() -> Self {
        // TODO is there a better way of doing this ?
        Self::new(bindings::create_comment(intern("")).into())
    }

    #[inline]
    pub fn with_state<A, F>(mut state: A, initializer: F) -> Dom
        where A: 'static,
              F: FnOnce(&mut A) -> Dom {

        let mut dom = initializer(&mut state);

        dom.callbacks.after_remove(ValueDiscard::new(state));

        dom
    }
}


#[inline]
fn create_element<A>(name: &str) -> A where A: JsCast {
    bindings::create_element(intern(name)).dyn_into().unwrap_throw()
}

#[inline]
fn create_element_ns<A>(name: &str, namespace: &str) -> A where A: JsCast {
    bindings::create_element_ns(intern(namespace), intern(name)).dyn_into().unwrap_throw()
}


fn set_option<A, B, C, D, F>(element: A, callbacks: &mut Callbacks, value: D, mut f: F)
    where A: 'static,
          C: OptionStr<Output = B>,
          D: Signal<Item = C> + 'static,
          F: FnMut(&A, Option<B>) + 'static {

    let mut is_set = false;

    callbacks.after_remove(for_each(value, move |value| {
        let value = value.into_option();

        if value.is_some() {
            is_set = true;

        } else if is_set {
            is_set = false;

        } else {
            return;
        }

        f(&element, value);
    }));
}

fn set_style<A, B>(style: &CssStyleDeclaration, name: &A, value: B, important: bool)
    where A: MultiStr,
          B: MultiStr {

    let mut names = vec![];
    let mut values = vec![];

    fn try_set_style(style: &CssStyleDeclaration, names: &mut Vec<String>, values: &mut Vec<String>, name: &str, value: &str, important: bool) -> Option<()> {
        assert!(value != "");

        // TODO handle browser prefixes ?
        bindings::remove_style(style, name);

        bindings::set_style(style, name, value, important);

        // TODO maybe use cfg(debug_assertions) ?
        let is_changed = bindings::get_style(style, name) != "";

        if is_changed {
            Some(())

        } else {
            names.push(String::from(name));
            values.push(String::from(value));
            None
        }
    }

    let okay = name.find_map(|name| {
        let name: &str = intern(name);

        value.find_map(|value| {
            // TODO should this intern ?
            try_set_style(style, &mut names, &mut values, &name, &value, important)
        })
    });

    if let None = okay {
        // TODO maybe make this configurable
        panic!("style is incorrect:\n  names: {}\n  values: {}", names.join(", "), values.join(", "));
    }
}

fn set_style_signal<A, B, C, D>(style: CssStyleDeclaration, callbacks: &mut Callbacks, name: A, value: D, important: bool)
    where A: MultiStr + 'static,
          B: MultiStr,
          C: OptionStr<Output = B>,
          D: Signal<Item = C> + 'static {

    set_option(style, callbacks, value, move |style, value| {
        match value {
            Some(value) => {
                // TODO should this intern or not ?
                set_style(style, &name, value, important);
            },
            None => {
                name.each(|name| {
                    // TODO handle browser prefixes ?
                    bindings::remove_style(style, intern(name));
                });
            },
        }
    });
}

// TODO check that the property *actually* was changed ?
// TODO maybe use AsRef<Object> ?
fn set_property<A, B, C>(element: &A, name: &B, value: C) where A: AsRef<JsValue>, B: MultiStr, C: Into<JsValue> {
    let element = element.as_ref();
    let value = value.into();

    name.each(|name| {
        bindings::set_property(element, intern(name), &value);
    });
}


// TODO better warning message for must_use
#[must_use]
pub struct DomBuilder<A> {
    element: A,
    callbacks: Callbacks,
    // TODO verify this with static types instead ?
    has_children: bool,
}

impl<A> DomBuilder<A> where A: JsCast {
    #[inline]
    pub fn new_html(name: &str) -> Self {
        Self::new(create_element(name))
    }

    #[inline]
    pub fn new_svg(name: &str) -> Self {
        Self::new(create_element_ns(name, SVG_NAMESPACE))
    }
}

impl<A> DomBuilder<A> {
    #[inline]
    pub fn new(value: A) -> Self {
        Self {
            element: value,
            callbacks: Callbacks::new(),
            has_children: false,
        }
    }

    #[inline]
    fn _event<T, F>(&mut self, element: EventTarget, listener: F)
        where T: StaticEvent,
              F: FnMut(T) + 'static {
        self.callbacks.after_remove(on(element, listener));
    }

    #[inline]
    fn _event_preventable<T, F>(&mut self, element: EventTarget, listener: F)
        where T: StaticEvent,
              F: FnMut(T) + 'static {
        self.callbacks.after_remove(on_preventable(element, listener));
    }

    // TODO add this to the StylesheetBuilder and ClassBuilder too
    #[inline]
    pub fn global_event<T, F>(mut self, listener: F) -> Self
        where T: StaticEvent,
              F: FnMut(T) + 'static {
        self._event(bindings::window_event_target(), listener);
        self
    }

    // TODO add this to the StylesheetBuilder and ClassBuilder too
    #[inline]
    pub fn global_event_preventable<T, F>(mut self, listener: F) -> Self
        where T: StaticEvent,
              F: FnMut(T) + 'static {
        self._event_preventable(bindings::window_event_target(), listener);
        self
    }

    #[inline]
    pub fn future<F>(mut self, future: F) -> Self where F: Future<Output = ()> + 'static {
        self.callbacks.after_remove(DiscardOnDrop::leak(spawn_future(future)));
        self
    }


    #[inline]
    pub fn apply<F>(self, f: F) -> Self where F: FnOnce(Self) -> Self {
        self.apply_if(true, f)
    }

    #[inline]
    pub fn apply_if<F>(self, test: bool, f: F) -> Self where F: FnOnce(Self) -> Self {
        if test {
            f(self)

        } else {
            self
        }
    }
}

impl<A> DomBuilder<A> where A: Clone {
    #[inline]
    #[doc(hidden)]
    pub fn __internal_element(&self) -> A {
        self.element.clone()
    }

    #[deprecated(since = "0.5.1", note = "Use the with_node macro instead")]
    #[inline]
    pub fn with_element<B, F>(self, f: F) -> B where F: FnOnce(Self, A) -> B {
        let element = self.element.clone();
        f(self, element)
    }

    #[inline]
    pub fn before_inserted<F>(self, f: F) -> Self where F: FnOnce(A) {
        let element = self.element.clone();
        f(element);
        self
    }
}

impl<A> DomBuilder<A> where A: Clone + 'static {
    #[inline]
    pub fn after_inserted<F>(mut self, f: F) -> Self where F: FnOnce(A) + 'static {
        let element = self.element.clone();
        self.callbacks.after_insert(move |_| f(element));
        self
    }

    #[inline]
    pub fn after_removed<F>(mut self, f: F) -> Self where F: FnOnce(A) + 'static {
        let element = self.element.clone();
        self.callbacks.after_remove(FnDiscard::new(move || f(element)));
        self
    }
}

impl<A> DomBuilder<A> where A: Into<Node> {
    #[inline]
    pub fn into_dom(self) -> Dom {
        Dom {
            element: self.element.into(),
            callbacks: self.callbacks,
        }
    }
}

impl<A> DomBuilder<A> where A: AsRef<JsValue> {
    #[inline]
    pub fn property<B, C>(self, name: B, value: C) -> Self where B: MultiStr, C: Into<JsValue> {
        set_property(&self.element, &name, value);
        self
    }
}

impl<A> DomBuilder<A> where A: AsRef<JsValue> {
    fn set_property_signal<B, C, D>(&mut self, name: B, value: D)
        where B: MultiStr + 'static,
              C: Into<JsValue>,
              D: Signal<Item = C> + 'static {

        let element = self.element.as_ref().clone();

        self.callbacks.after_remove(for_each(value, move |value| {
            set_property(&element, &name, value);
        }));
    }

    #[inline]
    pub fn property_signal<B, C, D>(mut self, name: B, value: D) -> Self
        where B: MultiStr + 'static,
              C: Into<JsValue>,
              D: Signal<Item = C> + 'static {

        self.set_property_signal(name, value);
        self
    }
}

impl<A> DomBuilder<A> where A: AsRef<EventTarget> {
    #[inline]
    pub fn event<T, F>(mut self, listener: F) -> Self
        where T: StaticEvent,
              F: FnMut(T) + 'static {
        // TODO can this clone be avoided ?
        self._event(self.element.as_ref().clone(), listener);
        self
    }

    #[inline]
    pub fn event_preventable<T, F>(mut self, listener: F) -> Self
        where T: StaticEvent,
              F: FnMut(T) + 'static {
        // TODO can this clone be avoided ?
        self._event_preventable(self.element.as_ref().clone(), listener);
        self
    }
}

impl<A> DomBuilder<A> where A: AsRef<Node> {
    #[inline]
    fn check_children(&mut self) {
        assert_eq!(self.has_children, false);
        self.has_children = true;
    }

    // TODO figure out how to make this owned rather than &mut
    #[inline]
    pub fn children<'a, B: IntoIterator<Item = &'a mut Dom>>(mut self, children: B) -> Self {
        self.check_children();
        operations::insert_children_iter(self.element.as_ref(), &mut self.callbacks, children);
        self
    }

    #[inline]
    pub fn text(mut self, value: &str) -> Self {
        self.check_children();
        // TODO should this intern ?
        bindings::set_text_content(self.element.as_ref(), &value);
        self
    }

    fn set_text_signal<B, C>(&mut self, value: C)
        where B: AsStr,
              C: Signal<Item = B> + 'static {

        let element = self.element.as_ref().clone();

        self.callbacks.after_remove(for_each(value, move |value| {
            let value = value.as_str();
            // TODO maybe intern this ?
            bindings::set_text_content(&element, &value);
        }));
    }

    #[inline]
    pub fn text_signal<B, C>(mut self, value: C) -> Self
        where B: AsStr,
              C: Signal<Item = B> + 'static {

        self.check_children();
        self.set_text_signal(value);
        self
    }

    #[inline]
    pub fn children_signal_vec<B>(mut self, children: B) -> Self
        where B: SignalVec<Item = Dom> + 'static {

        self.check_children();
        operations::insert_children_signal_vec(self.element.as_ref().clone(), &mut self.callbacks, children);
        self
    }

    #[inline]
    pub fn child_signal<B>(mut self, child: B) -> Self
        where B: Signal<Item = Option<Dom>> + 'static {

        self.check_children();
        operations::insert_child_signal(self.element.as_ref().clone(), &mut self.callbacks, child);
        self
    }
}

impl<A> DomBuilder<A> where A: AsRef<Element> {
    #[inline]
    pub fn attribute<B>(self, name: B, value: &str) -> Self where B: MultiStr {
        let element = self.element.as_ref();
        // TODO should this intern the value ?
        let value: &str = intern(value);

        name.each(|name| {
            bindings::set_attribute(element, intern(name), &value);
        });

        self
    }

    #[inline]
    pub fn attribute_namespace<B>(self, namespace: &str, name: B, value: &str) -> Self where B: MultiStr {
        let element = self.element.as_ref();
        let namespace: &str = intern(namespace);
        // TODO should this intern the value ?
        let value: &str = intern(value);

        name.each(|name| {
            bindings::set_attribute_ns(element, &namespace, intern(name), &value);
        });

        self
    }

    #[inline]
    pub fn class<B>(self, name: B) -> Self where B: MultiStr {
        let classes = self.element.as_ref().class_list();

        name.each(|name| {
            bindings::add_class(&classes, intern(name));
        });

        self
    }

    // TODO make this more efficient ?
    #[inline]
    pub fn visible(self, value: bool) -> Self {
        if value {
            // TODO remove the class somehow ?
            self

        } else {
            self.class(&*HIDDEN_CLASS)
        }
    }
}

impl<A> DomBuilder<A> where A: AsRef<Element> {
    fn set_attribute_signal<B, C, D, E>(&mut self, name: B, value: E)
        where B: MultiStr + 'static,
              C: AsStr,
              D: OptionStr<Output = C>,
              E: Signal<Item = D> + 'static {

        set_option(self.element.as_ref().clone(), &mut self.callbacks, value, move |element, value| {
            match value {
                Some(value) => {
                    let value = value.as_str();

                    name.each(|name| {
                        // TODO should this intern the value ?
                        bindings::set_attribute(element, intern(name), &value);
                    });
                },
                None => {
                    name.each(|name| {
                        bindings::remove_attribute(element, intern(name));
                    });
                },
            }
        });
    }


    #[inline]
    pub fn attribute_signal<B, C, D, E>(mut self, name: B, value: E) -> Self
        where B: MultiStr + 'static,
              C: AsStr,
              D: OptionStr<Output = C>,
              E: Signal<Item = D> + 'static {

        self.set_attribute_signal(name, value);
        self
    }

    fn set_attribute_namespace_signal<B, C, D, E>(&mut self, namespace: &str, name: B, value: E)
        where B: MultiStr + 'static,
              C: AsStr,
              D: OptionStr<Output = C>,
              E: Signal<Item = D> + 'static {

        // TODO avoid this to_owned by using Into<Cow<'static str>>
        let namespace: String = intern(namespace).to_owned();

        set_option(self.element.as_ref().clone(), &mut self.callbacks, value, move |element, value| {
            match value {
                Some(value) => {
                    let value = value.as_str();

                    name.each(|name| {
                        // TODO should this intern the value ?
                        bindings::set_attribute_ns(element, &namespace, intern(name), &value);
                    });
                },
                None => {
                    name.each(|name| {
                        bindings::remove_attribute_ns(element, &namespace, intern(name));
                    });
                },
            }
        });
    }

    #[inline]
    pub fn attribute_namespace_signal<B, C, D, E>(mut self, namespace: &str, name: B, value: E) -> Self
        where B: MultiStr + 'static,
              C: AsStr,
              D: OptionStr<Output = C>,
              E: Signal<Item = D> + 'static {

        self.set_attribute_namespace_signal(namespace, name, value);
        self
    }


    fn set_class_signal<B, C>(&mut self, name: B, value: C)
        where B: MultiStr + 'static,
              C: Signal<Item = bool> + 'static {

        let element = self.element.as_ref().class_list();

        let mut is_set = false;

        self.callbacks.after_remove(for_each(value, move |value| {
            if value {
                if !is_set {
                    is_set = true;

                    name.each(|name| {
                        bindings::add_class(&element, intern(name));
                    });
                }

            } else {
                if is_set {
                    is_set = false;

                    name.each(|name| {
                        bindings::remove_class(&element, intern(name));
                    });
                }
            }
        }));
    }

    #[inline]
    pub fn class_signal<B, C>(mut self, name: B, value: C) -> Self
        where B: MultiStr + 'static,
              C: Signal<Item = bool> + 'static {

        self.set_class_signal(name, value);
        self
    }

    // TODO make this more efficient ?
    #[inline]
    pub fn visible_signal<B>(self, value: B) -> Self where B: Signal<Item = bool> + 'static {
        self.class_signal(&*HIDDEN_CLASS, not(value))
    }


    // TODO use OptionStr ?
    fn set_scroll_signal<B, F>(&mut self, signal: B, mut f: F)
        where B: Signal<Item = Option<i32>> + 'static,
              F: FnMut(&Element, i32) + 'static {

        let element: Element = self.element.as_ref().clone();

        // This needs to use `after_insert` because scrolling an element before it is in the DOM has no effect
        self.callbacks.after_insert(move |callbacks| {
            callbacks.after_remove(for_each(signal, move |value| {
                if let Some(value) = value {
                    f(&element, value);
                }
            }));
        });
    }

    #[inline]
    pub fn scroll_left_signal<B>(mut self, signal: B) -> Self where B: Signal<Item = Option<i32>> + 'static {
        // TODO bindings function for this ?
        self.set_scroll_signal(signal, Element::set_scroll_left);
        self
    }

    #[inline]
    pub fn scroll_top_signal<B>(mut self, signal: B) -> Self where B: Signal<Item = Option<i32>> + 'static {
        // TODO bindings function for this ?
        self.set_scroll_signal(signal, Element::set_scroll_top);
        self
    }
}

impl<A> DomBuilder<A> where A: AsRef<HtmlElement> {
    #[inline]
    pub fn style<B, C>(self, name: B, value: C) -> Self
        where B: MultiStr,
              C: MultiStr {
        set_style(&self.element.as_ref().style(), &name, value, false);
        self
    }

    #[inline]
    pub fn style_important<B, C>(self, name: B, value: C) -> Self
        where B: MultiStr,
              C: MultiStr {
        set_style(&self.element.as_ref().style(), &name, value, true);
        self
    }
}

impl<A> DomBuilder<A> where A: AsRef<HtmlElement> {
    #[inline]
    pub fn style_signal<B, C, D, E>(mut self, name: B, value: E) -> Self
        where B: MultiStr + 'static,
              C: MultiStr,
              D: OptionStr<Output = C>,
              E: Signal<Item = D> + 'static {

        set_style_signal(self.element.as_ref().style(), &mut self.callbacks, name, value, false);
        self
    }

    #[inline]
    pub fn style_important_signal<B, C, D, E>(mut self, name: B, value: E) -> Self
        where B: MultiStr + 'static,
              C: MultiStr,
              D: OptionStr<Output = C>,
              E: Signal<Item = D> + 'static {

        set_style_signal(self.element.as_ref().style(), &mut self.callbacks, name, value, true);
        self
    }


    // TODO remove the `value` argument ?
    #[inline]
    pub fn focused(mut self, value: bool) -> Self {
        let element = self.element.as_ref().clone();

        // This needs to use `after_insert` because calling `.focus()` on an element before it is in the DOM has no effect
        self.callbacks.after_insert(move |_| {
            // TODO avoid updating if the focused state hasn't changed ?
            if value {
                bindings::focus(&element);

            } else {
                bindings::blur(&element);
            }
        });

        self
    }


    fn set_focused_signal<B>(&mut self, value: B)
        where B: Signal<Item = bool> + 'static {

        let element = self.element.as_ref().clone();

        // This needs to use `after_insert` because calling `.focus()` on an element before it is in the DOM has no effect
        self.callbacks.after_insert(move |callbacks| {
            // TODO verify that this is correct under all circumstances
            callbacks.after_remove(for_each(value, move |value| {
                // TODO avoid updating if the focused state hasn't changed ?
                if value {
                    bindings::focus(&element);

                } else {
                    bindings::blur(&element);
                }
            }));
        });
    }

    #[inline]
    pub fn focused_signal<B>(mut self, value: B) -> Self
        where B: Signal<Item = bool> + 'static {

        self.set_focused_signal(value);
        self
    }
}


// TODO better warning message for must_use
#[must_use]
pub struct StylesheetBuilder {
    element: CssStyleDeclaration,
    callbacks: Callbacks,
}

// TODO remove the CssStyleRule when this is discarded
impl StylesheetBuilder {
    // TODO should this inline ?
    #[doc(hidden)]
    #[inline]
    pub fn __internal_new<A>(selector: A) -> Self where A: MultiStr {
        // TODO can this be made faster ?
        // TODO somehow share this safely between threads ?
        thread_local! {
            static STYLESHEET: CssStyleSheet = bindings::create_stylesheet();
        }

        fn try_make(stylesheet: &CssStyleSheet, selector: &str, selectors: &mut Vec<String>) -> Option<CssStyleDeclaration> {
            // TODO maybe intern the selector ?
            if let Ok(declaration) = bindings::make_style_rule(stylesheet, selector) {
                Some(declaration.style())

            } else {
                selectors.push(String::from(selector));
                None
            }
        }

        let element = STYLESHEET.with(move |stylesheet| {
            let mut selectors = vec![];

            let okay = selector.find_map(|selector| {
                try_make(stylesheet, selector, &mut selectors)
            });

            if let Some(okay) = okay {
                okay

            } else {
                // TODO maybe make this configurable
                panic!("selectors are incorrect:\n  {}", selectors.join("\n  "));
            }
        });

        Self {
            element,
            callbacks: Callbacks::new(),
        }
    }

    #[inline]
    pub fn style<B, C>(self, name: B, value: C) -> Self
        where B: MultiStr,
              C: MultiStr {
        set_style(&self.element, &name, value, false);
        self
    }

    #[inline]
    pub fn style_important<B, C>(self, name: B, value: C) -> Self
        where B: MultiStr,
              C: MultiStr {
        set_style(&self.element, &name, value, true);
        self
    }

    #[inline]
    pub fn style_signal<B, C, D, E>(mut self, name: B, value: E) -> Self
        where B: MultiStr + 'static,
              C: MultiStr,
              D: OptionStr<Output = C>,
              E: Signal<Item = D> + 'static {

        set_style_signal(self.element.clone(), &mut self.callbacks, name, value, false);
        self
    }

    #[inline]
    pub fn style_important_signal<B, C, D, E>(mut self, name: B, value: E) -> Self
        where B: MultiStr + 'static,
              C: MultiStr,
              D: OptionStr<Output = C>,
              E: Signal<Item = D> + 'static {

        set_style_signal(self.element.clone(), &mut self.callbacks, name, value, true);
        self
    }

    // TODO return a Handle
    #[inline]
    #[doc(hidden)]
    pub fn __internal_done(mut self) {
        self.callbacks.trigger_after_insert();

        // This prevents it from triggering after_remove
        self.callbacks.leak();
    }
}


// TODO better warning message for must_use
#[must_use]
pub struct ClassBuilder {
    stylesheet: StylesheetBuilder,
    class_name: String,
}

impl ClassBuilder {
    #[doc(hidden)]
    #[inline]
    pub fn __internal_new() -> Self {
        let class_name = __internal::make_class_id();

        Self {
            // TODO make this more efficient ?
            stylesheet: StylesheetBuilder::__internal_new(&format!(".{}", class_name)),
            class_name,
        }
    }

    #[doc(hidden)]
    #[inline]
    pub fn __internal_class_name(&self) -> &str {
        &self.class_name
    }

    #[inline]
    pub fn style<B, C>(mut self, name: B, value: C) -> Self
        where B: MultiStr,
              C: MultiStr {
        self.stylesheet = self.stylesheet.style(name, value);
        self
    }

    #[inline]
    pub fn style_important<B, C>(mut self, name: B, value: C) -> Self
        where B: MultiStr,
              C: MultiStr {
        self.stylesheet = self.stylesheet.style_important(name, value);
        self
    }

    #[inline]
    pub fn style_signal<B, C, D, E>(mut self, name: B, value: E) -> Self
        where B: MultiStr + 'static,
              C: MultiStr,
              D: OptionStr<Output = C>,
              E: Signal<Item = D> + 'static {

        self.stylesheet = self.stylesheet.style_signal(name, value);
        self
    }

    #[inline]
    pub fn style_important_signal<B, C, D, E>(mut self, name: B, value: E) -> Self
        where B: MultiStr + 'static,
              C: MultiStr,
              D: OptionStr<Output = C>,
              E: Signal<Item = D> + 'static {

        self.stylesheet = self.stylesheet.style_important_signal(name, value);
        self
    }

    // TODO return a Handle ?
    #[doc(hidden)]
    #[inline]
    pub fn __internal_done(self) -> String {
        self.stylesheet.__internal_done();
        self.class_name
    }
}


#[doc(hidden)]
pub mod __internal {
    use std::sync::atomic::{AtomicU32, Ordering};
    use crate::traits::MultiStr;


    pub use web_sys::HtmlElement;
    pub use web_sys::SvgElement;


    pub fn make_class_id() -> String {
        // TODO replace this with a global counter in JavaScript ?
        // TODO can this be made more efficient ?
        static CLASS_ID: AtomicU32 = AtomicU32::new(0);

        // TODO check for overflow ?
        // TODO should this be SeqCst ?
        let id = CLASS_ID.fetch_add(1, Ordering::Relaxed);

        // TODO make this more efficient ?
        format!("__class_{}__", id)
    }


    pub struct Pseudo<'a, A> {
        class_name: &'a str,
        pseudos: A,
    }

    impl<'a, A> Pseudo<'a, A> where A: MultiStr {
        #[inline]
        pub fn new(class_name: &'a str, pseudos: A) -> Self {
            Self { class_name, pseudos }
        }
    }

    impl<'a, A> MultiStr for Pseudo<'a, A> where A: MultiStr {
        #[inline]
        fn find_map<B, F>(&self, mut f: F) -> Option<B> where F: FnMut(&str) -> Option<B> {
            self.pseudos.find_map(|x| {
                f(&format!(".{}{}", self.class_name, x))
            })
        }
    }
}


#[cfg(test)]
mod tests {
    use super::{DomBuilder, text_signal, RefFn};
    use futures_signals::signal::{always, SignalExt};
    use lazy_static::lazy_static;
    use web_sys::HtmlElement;

    #[test]
    fn apply() {
        let a: DomBuilder<HtmlElement> = DomBuilder::new_html("div");

        fn my_mixin<A: AsRef<HtmlElement>>(builder: DomBuilder<A>) -> DomBuilder<A> {
            builder.style("foo", "bar")
        }

        let _ = a.apply(my_mixin);
    }

    #[test]
    fn text_signal_types() {
        let _ = text_signal(always("foo"));
        let _ = text_signal(always("foo".to_owned()));
        let _ = text_signal(always("foo".to_owned()).map(|x| RefFn::new(x, |x| x.as_str())));
        //text_signal(always(Arc::new("foo")));
        //text_signal(always(Arc::new("foo".to_owned())));
        //text_signal(always(Rc::new("foo")));
        //text_signal(always(Rc::new("foo".to_owned())));
        //text_signal(always(Box::new("foo")));
        //text_signal(always(Box::new("foo".to_owned())));
        //text_signal(always(Cow::Borrowed(&"foo")));
        //text_signal(always(Cow::Owned::<String>("foo".to_owned())));
    }

    #[test]
    fn property_signal_types() {
        let _a: DomBuilder<HtmlElement> = DomBuilder::new_html("div")
            .property("foo", "hi")
            .property("foo", 5)
            .property(["foo", "-webkit-foo", "-ms-foo"], "hi")

            .property_signal("foo", always("hi"))
            .property_signal("foo", always(5))
            .property_signal("foo", always(Some("hi")))

            .property_signal(["foo", "-webkit-foo", "-ms-foo"], always("hi"))
            .property_signal(["foo", "-webkit-foo", "-ms-foo"], always(5))
            .property_signal(["foo", "-webkit-foo", "-ms-foo"], always(Some("hi")))
            ;
    }

    #[test]
    fn attribute_signal_types() {
        let _a: DomBuilder<HtmlElement> = DomBuilder::new_html("div")
            .attribute("foo", "hi")
            .attribute(["foo", "-webkit-foo", "-ms-foo"], "hi")

            .attribute_signal("foo", always("hi"))
            .attribute_signal("foo", always(Some("hi")))

            .attribute_signal(["foo", "-webkit-foo", "-ms-foo"], always("hi"))
            .attribute_signal(["foo", "-webkit-foo", "-ms-foo"], always(Some("hi")))
            ;
    }

    #[test]
    fn class_signal_types() {
        let _a: DomBuilder<HtmlElement> = DomBuilder::new_html("div")
            .class("foo")
            .class(["foo", "-webkit-foo", "-ms-foo"])

            .class_signal("foo", always(true))
            .class_signal(["foo", "-webkit-foo", "-ms-foo"], always(true))
            ;
    }

    #[test]
    fn style_signal_types() {
        lazy_static! {
            static ref FOO: String = "foo".to_owned();
        }

        let _a: DomBuilder<HtmlElement> = DomBuilder::new_html("div")
            .style_signal("foo", always("bar"))
            .style_signal("foo", always("bar".to_owned()))
            .style_signal("foo", always("bar".to_owned()).map(|x| RefFn::new(x, |x| x.as_str())))

            .style("foo".to_owned(), "bar".to_owned())
            .style_signal("foo".to_owned(), always("bar".to_owned()))

            .style(&"foo".to_owned(), &"bar".to_owned())
            //.style(Box::new("foo".to_owned()), Box::new("bar".to_owned()))
            //.style_signal(Box::new("foo".to_owned()), always(Box::new("bar".to_owned())))

            .style_signal(&*FOO, always(&*FOO))

            //.style(vec!["-moz-foo", "-webkit-foo", "foo"].as_slice(), vec!["bar"].as_slice())
            .style_signal(RefFn::new(vec!["-moz-foo", "-webkit-foo", "foo"], |x| x.as_slice()), always(RefFn::new(vec!["bar"], |x| x.as_slice())))

            .style_signal(["-moz-foo", "-webkit-foo", "foo"], always("bar"))
            .style_signal(["-moz-foo", "-webkit-foo", "foo"], always("bar".to_owned()))
            .style_signal(["-moz-foo", "-webkit-foo", "foo"], always("bar".to_owned()).map(|x| RefFn::new(x, |x| x.as_str())))

            .style_signal(["-moz-foo", "-webkit-foo", "foo"], always(["bar", "qux"]))
            .style_signal(["-moz-foo", "-webkit-foo", "foo"], always(["bar".to_owned(), "qux".to_owned()]))

            //.style_signal(["-moz-foo", "-webkit-foo", "foo"], always(AsSlice::new(["foo", "bar"])))
            //.style_signal(["-moz-foo", "-webkit-foo", "foo"], always(("bar".to_owned(), "qux".to_owned())).map(|x| RefFn::new(x, |x| AsSlice::new([x.0.as_str(), x.1.as_str()]))))

            .style_signal("foo", always(Some("bar")))
            .style_signal("foo", always(Some("bar".to_owned())))
            .style_signal("foo", always("bar".to_owned()).map(|x| Some(RefFn::new(x, |x| x.as_str()))))

            .style_signal(["-moz-foo", "-webkit-foo", "foo"], always(Some("bar")))
            .style_signal(["-moz-foo", "-webkit-foo", "foo"], always(Some("bar".to_owned())))
            .style_signal(["-moz-foo", "-webkit-foo", "foo"], always("bar".to_owned()).map(|x| Some(RefFn::new(x, |x| x.as_str()))))
            ;
    }
}
