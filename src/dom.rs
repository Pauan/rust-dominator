use stdweb::{Reference, Value, JsSerialize};
use stdweb::unstable::{TryFrom, TryInto};
use stdweb::web::{IEventTarget, INode, IElement, IHtmlElement, HtmlElement, Node, window, TextNode, EventTarget, Element};
use stdweb::web::event::ConcreteEvent;
use callbacks::Callbacks;
use traits::*;
use operations;
use operations::for_each;
use dom_operations;
use operations::{ValueDiscard, FnDiscard, spawn_future};
use futures_signals::signal::IntoSignal;
use futures_signals::signal_vec::IntoSignalVec;
use futures_core::Never;
use futures_core::future::Future;
use discard::{Discard, DiscardOnDrop};


// TODO this should be in stdweb
#[derive(Clone, Debug, PartialEq, Eq, ReferenceType)]
#[reference(instance_of = "CSSStyleRule")]
pub struct CssStyleRule(Reference);


/// A reference to an SVG Element.
///
/// [(JavaScript docs)](https://developer.mozilla.org/en-US/docs/Web/API/SVGElement)
#[derive(Clone, Debug, PartialEq, Eq, ReferenceType)]
#[reference(instance_of = "SVGElement")]
#[reference(subclass_of(EventTarget, Node, Element))]
pub struct SvgElement(Reference);


// https://developer.mozilla.org/en-US/docs/Web/API/Document/createElementNS#Valid%20Namespace%20URIs
pub const HTML_NAMESPACE: &str = "http://www.w3.org/1999/xhtml";
pub const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";

// 32-bit signed int
pub const HIGHEST_ZINDEX: &str = "2147483647";


// TODO this should be in stdweb
// TODO this should return HtmlBodyElement
pub fn body() -> HtmlElement {
    js! ( return document.body; ).try_into().unwrap()
}


pub struct DomHandle {
    parent: Node,
    dom: Dom,
}

impl Discard for DomHandle {
    #[inline]
    fn discard(self) {
        self.parent.remove_child(&self.dom.element).unwrap();
        self.dom.callbacks.discard();
    }
}

#[inline]
pub fn append_dom<A: INode>(parent: &A, mut dom: Dom) -> DomHandle {
    parent.append_child(&dom.element);

    dom.callbacks.trigger_after_insert();

    // This prevents it from triggering after_remove
    dom.callbacks.leak();

    DomHandle {
        parent: parent.as_node().clone(),
        dom
    }
}


#[inline]
pub fn text(value: &str) -> Dom {
    Dom::new(js!( return document.createTextNode(@{value}); ).try_into().unwrap())
}


// TODO should this inline ?
pub fn text_signal<A, B>(value: B) -> Dom
    where A: AsStr,
          B: IntoSignal<Item = A>,
          B::Signal: 'static {

    let element: TextNode = js!( return document.createTextNode(""); ).try_into().unwrap();

    let mut callbacks = Callbacks::new();

    {
        let element = element.clone();

        callbacks.after_remove(for_each(value.into_signal(), move |value| {
            dom_operations::set_text(&element, value.as_str());
        }));
    }

    Dom {
        element: element.into(),
        callbacks: callbacks,
    }
}


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
        Self::new(js!( return document.createComment(""); ).try_into().unwrap())
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


struct EventListenerHandle<A> where A: AsRef<Reference> {
    event: &'static str,
    element: A,
    listener: Value,
}

impl<A> Discard for EventListenerHandle<A> where A: AsRef<Reference> {
    #[inline]
    fn discard(self) {
        js! { @(no_return)
            var listener = @{&self.listener};
            @{self.element.as_ref()}.removeEventListener(@{self.event}, listener);
            listener.drop();
        }
    }
}


// TODO create HTML / SVG specific versions of this ?
#[inline]
pub fn create_element_ns<A: IElement>(name: &str, namespace: &str) -> A
    where <A as TryFrom<Value>>::Error: ::std::fmt::Debug {
    dom_operations::create_element_ns(name, namespace)
}


fn set_option_str<A, B, C, F>(element: &A, callbacks: &mut Callbacks, value: C, mut f: F)
    where A: Clone + 'static,
          B: AsOptionStr,
          C: IntoSignal<Item = B>,
          C::Signal: 'static,
          F: FnMut(&A, Option<&str>) + 'static {

    let element = element.clone();

    let mut is_set = false;

    callbacks.after_remove(for_each(value.into_signal(), move |value| {
        let value = value.as_option_str();

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


pub struct DomBuilder<A> {
    element: A,
    callbacks: Callbacks,
    // TODO verify this with static types instead ?
    has_children: bool,
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

    // TODO maybe inline this ?
    // TODO replace with element.add_event_listener
    fn _event<B, T, F>(&mut self, element: B, listener: F)
        where B: IEventTarget + 'static,
              T: ConcreteEvent,
              F: FnMut(T) + 'static {

        let listener = js!(
            var listener = @{listener};
            @{element.as_ref()}.addEventListener(@{T::EVENT_TYPE}, listener);
            return listener;
        );

        self.callbacks.after_remove(EventListenerHandle {
            event: T::EVENT_TYPE,
            element,
            listener,
        });
    }

    // TODO add this to the StylesheetBuilder and ClassBuilder too
    #[inline]
    pub fn global_event<T, F>(mut self, listener: F) -> Self
        where T: ConcreteEvent,
              F: FnMut(T) + 'static {
        self._event(window(), listener);
        self
    }

    #[inline]
    pub fn future<F>(mut self, future: F) -> Self where F: Future<Item = (), Error = Never> + 'static {
        self.callbacks.after_remove(DiscardOnDrop::leak(spawn_future(future)));
        self
    }

    #[inline]
    pub fn mixin<B: Mixin<Self>>(self, mixin: B) -> Self {
        mixin.apply(self)
    }
}

impl<A: Clone> DomBuilder<A> {
    #[inline]
    pub fn before_inserted<F>(self, f: F) -> Self where F: FnOnce(A) {
        let element = self.element.clone();
        f(element);
        self
    }
}

impl<A: Clone + 'static> DomBuilder<A> {
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

impl<A: Into<Node>> DomBuilder<A> {
    #[inline]
    pub fn into_dom(self) -> Dom {
        Dom {
            element: self.element.into(),
            callbacks: self.callbacks,
        }
    }
}

// TODO make this JsSerialize rather than AsRef<Reference> ?
impl<A: AsRef<Reference>> DomBuilder<A> {
    #[inline]
    pub fn property<B: JsSerialize>(self, name: &str, value: B) -> Self {
        dom_operations::set_property(&self.element, name, value);
        self
    }
}

impl<A: AsRef<Reference> + Clone + 'static> DomBuilder<A> {
    fn set_property_signal<B, C>(&mut self, name: &str, value: C)
        where B: JsSerialize,
              C: IntoSignal<Item = B>,
              C::Signal: 'static {

        let element = self.element.clone();
        let name = name.to_owned();

        self.callbacks.after_remove(for_each(value.into_signal(), move |value| {
            dom_operations::set_property(&element, &name, value);
        }));
    }

    #[inline]
    pub fn property_signal<B, C>(mut self, name: &str, value: C) -> Self
        where B: JsSerialize,
              C: IntoSignal<Item = B>,
              C::Signal: 'static {

        self.set_property_signal(name, value);
        self
    }
}

impl<A: IEventTarget + Clone + 'static> DomBuilder<A> {
    #[inline]
    pub fn event<T, F>(mut self, listener: F) -> Self
        where T: ConcreteEvent,
              F: FnMut(T) + 'static {
        // TODO is this clone correct ?
        let element = self.element.clone();
        self._event(element, listener);
        self
    }
}

impl<A: INode> DomBuilder<A> {
    // TODO figure out how to make this owned rather than &mut
    #[inline]
    pub fn children<'a, B: IntoIterator<Item = &'a mut Dom>>(mut self, children: B) -> Self {
        assert_eq!(self.has_children, false);
        self.has_children = true;

        operations::insert_children_iter(&self.element, &mut self.callbacks, children);
        self
    }
}

impl<A: INode + Clone + 'static> DomBuilder<A> {
    #[inline]
    pub fn children_signal_vec<B>(mut self, children: B) -> Self
        where B: IntoSignalVec<Item = Dom>,
              B::SignalVec: 'static {

        assert_eq!(self.has_children, false);
        self.has_children = true;

        operations::insert_children_signal_vec(&self.element, &mut self.callbacks, children);
        self
    }
}

impl<A: IElement> DomBuilder<A> {
    #[inline]
    pub fn attribute(self, name: &str, value: &str) -> Self {
        dom_operations::set_attribute(&self.element, name, value);
        self
    }

    #[inline]
    pub fn attribute_namespace(self, namespace: &str, name: &str, value: &str) -> Self {
        dom_operations::set_attribute_ns(&self.element, namespace, name, value);
        self
    }

    #[inline]
    pub fn class(self, name: &str) -> Self {
        dom_operations::add_class(&self.element, name);
        self
    }
}

impl<A: IElement + Clone + 'static> DomBuilder<A> {
    fn set_attribute_signal<B, C>(&mut self, name: &str, value: C)
        where B: AsOptionStr,
              C: IntoSignal<Item = B>,
              C::Signal: 'static {

        let name = name.to_owned();

        set_option_str(&self.element, &mut self.callbacks, value, move |element, value| {
            match value {
                Some(value) => dom_operations::set_attribute(element, &name, value),
                None => dom_operations::remove_attribute(element, &name),
            }
        });
    }


    #[inline]
    pub fn attribute_signal<B, C>(mut self, name: &str, value: C) -> Self
        where B: AsOptionStr,
              C: IntoSignal<Item = B>,
              C::Signal: 'static {

        self.set_attribute_signal(name, value);
        self
    }

    fn set_attribute_namespace_signal<B, C>(&mut self, namespace: &str, name: &str, value: C)
        where B: AsOptionStr,
              C: IntoSignal<Item = B>,
              C::Signal: 'static {

        let name = name.to_owned();
        let namespace = namespace.to_owned();

        set_option_str(&self.element, &mut self.callbacks, value, move |element, value| {
            match value {
                Some(value) => dom_operations::set_attribute_ns(element, &namespace, &name, value),
                None => dom_operations::remove_attribute_ns(element, &namespace, &name),
            }
        });
    }

    #[inline]
    pub fn attribute_namespace_signal<B, C>(mut self, namespace: &str, name: &str, value: C) -> Self
        where B: AsOptionStr,
              C: IntoSignal<Item = B>,
              C::Signal: 'static {

        self.set_attribute_namespace_signal(namespace, name, value);
        self
    }


    fn set_class_signal<B>(&mut self, name: &str, value: B)
        where B: IntoSignal<Item = bool>,
              B::Signal: 'static {

        let element = self.element.clone();
        let name = name.to_owned();

        let mut is_set = false;

        self.callbacks.after_remove(for_each(value.into_signal(), move |value| {
            if value {
                if !is_set {
                    is_set = true;
                    dom_operations::add_class(&element, &name);
                }

            } else {
                if is_set {
                    is_set = false;
                    dom_operations::remove_class(&element, &name);
                }
            }
        }));
    }

    #[inline]
    pub fn class_signal<B>(mut self, name: &str, value: B) -> Self
        where B: IntoSignal<Item = bool>,
              B::Signal: 'static {

        self.set_class_signal(name, value);
        self
    }
}

impl<A: IHtmlElement> DomBuilder<A> {
    #[inline]
    pub fn style(self, name: &str, value: &str) -> Self {
        dom_operations::set_style(&self.element, name, value, false);
        self
    }

    #[inline]
    pub fn style_important(self, name: &str, value: &str) -> Self {
        dom_operations::set_style(&self.element, name, value, true);
        self
    }
}

impl<A: IHtmlElement + Clone + 'static> DomBuilder<A> {
    fn set_style_signal<B, C>(&mut self, name: &str, value: C, important: bool)
        where B: AsOptionStr,
              C: IntoSignal<Item = B>,
              C::Signal: 'static {

        let name = name.to_owned();

        set_option_str(&self.element, &mut self.callbacks, value, move |element, value| {
            dom_operations::set_style(element, &name, value.unwrap_or(""), important);
        });
    }

    #[inline]
    pub fn style_signal<B, C>(mut self, name: &str, value: C) -> Self
        where B: AsOptionStr,
              C: IntoSignal<Item = B>,
              C::Signal: 'static {

        self.set_style_signal(name, value, false);
        self
    }

    #[inline]
    pub fn style_important_signal<B, C>(mut self, name: &str, value: C) -> Self
        where B: AsOptionStr,
              C: IntoSignal<Item = B>,
              C::Signal: 'static {

        self.set_style_signal(name, value, true);
        self
    }


    // TODO remove the `value` argument ?
    #[inline]
    pub fn focused(mut self, value: bool) -> Self {
        let element = self.element.clone();

        // This needs to use `after_insert` because calling `.focus()` on an element before it is in the DOM has no effect
        self.callbacks.after_insert(move |_| {
            // TODO avoid updating if the focused state hasn't changed ?
            dom_operations::set_focused(&element, value);
        });

        self
    }


    fn set_focused_signal<B>(&mut self, value: B)
        where B: IntoSignal<Item = bool> + 'static {

        let element = self.element.clone();

        // This needs to use `after_insert` because calling `.focus()` on an element before it is in the DOM has no effect
        self.callbacks.after_insert(move |callbacks| {
            // TODO verify that this is correct under all circumstances
            callbacks.after_remove(for_each(value.into_signal(), move |value| {
                // TODO avoid updating if the focused state hasn't changed ?
                dom_operations::set_focused(&element, value);
            }));
        });
    }

    #[inline]
    pub fn focused_signal<B>(mut self, value: B) -> Self
        where B: IntoSignal<Item = bool> + 'static {

        self.set_focused_signal(value);
        self
    }
}


#[cfg(test)]
mod tests {
    use super::{create_element_ns, DomBuilder, HTML_NAMESPACE};
    use stdweb::web::{HtmlElement, IHtmlElement};

    #[test]
    fn mixin() {
        let a: DomBuilder<HtmlElement> = DomBuilder::new(create_element_ns("div", HTML_NAMESPACE));

        fn my_mixin<A: IHtmlElement>(builder: DomBuilder<A>) -> DomBuilder<A> {
            builder.style("foo", "bar")
        }

        a.mixin(my_mixin);
    }
}


// TODO better warning message for must_use
#[must_use]
pub struct StylesheetBuilder {
    element: CssStyleRule,
    callbacks: Callbacks,
}

// TODO remove the CssStyleRule when this is discarded
impl StylesheetBuilder {
    #[inline]
    pub fn new(selector: &str) -> Self {
        lazy_static! {
            // TODO better static type for this
            static ref STYLESHEET: Reference = js!(
                // TODO use createElementNS ?
                var e = document.createElement("style");
                e.type = "text/css";
                document.head.appendChild(e);
                return e.sheet;
            ).try_into().unwrap();
        }

        Self {
            element: js!(
                var stylesheet = @{&*STYLESHEET};
                var length = stylesheet.cssRules.length;
                stylesheet.insertRule(@{selector} + "{}", length);
                return stylesheet.cssRules[length];
            ).try_into().unwrap(),
            callbacks: Callbacks::new(),
        }
    }

    #[inline]
    pub fn style(self, name: &str, value: &str) -> Self {
        dom_operations::set_style(&self.element, name, value, false);
        self
    }

    #[inline]
    pub fn style_important(self, name: &str, value: &str) -> Self {
        dom_operations::set_style(&self.element, name, value, true);
        self
    }


    fn set_style_signal<B, C>(&mut self, name: &str, value: C, important: bool)
        where B: AsOptionStr,
              C: IntoSignal<Item = B>,
              C::Signal: 'static {

        let name = name.to_owned();

        set_option_str(&self.element, &mut self.callbacks, value, move |element, value| {
            dom_operations::set_style(element, &name, value.unwrap_or(""), important);
        });
    }

    #[inline]
    pub fn style_signal<B, C>(mut self, name: &str, value: C) -> Self
        where B: AsOptionStr,
              C: IntoSignal<Item = B>,
              C::Signal: 'static {

        self.set_style_signal(name, value, false);
        self
    }

    #[inline]
    pub fn style_important_signal<B, C>(mut self, name: &str, value: C) -> Self
        where B: AsOptionStr,
              C: IntoSignal<Item = B>,
              C::Signal: 'static {

        self.set_style_signal(name, value, true);
        self
    }

    // TODO return a Handle
    #[inline]
    pub fn done(mut self) {
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
    #[inline]
    pub fn new() -> Self {
        let class_name = {
            use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT, Ordering};

            // TODO replace this with a global counter in JavaScript ?
            lazy_static! {
                // TODO can this be made more efficient ?
                // TODO use AtomicU32 instead ?
                static ref CLASS_ID: AtomicUsize = ATOMIC_USIZE_INIT;
            }

            // TODO check for overflow ?
            let id = CLASS_ID.fetch_add(1, Ordering::Relaxed);

            // TODO make this more efficient ?
            format!("__class_{}__", id)
        };

        Self {
            // TODO make this more efficient ?
            stylesheet: StylesheetBuilder::new(&format!(".{}", class_name)),
            class_name,
        }
    }

    #[inline]
    pub fn style(mut self, name: &str, value: &str) -> Self {
        self.stylesheet = self.stylesheet.style(name, value);
        self
    }

    #[inline]
    pub fn style_important(mut self, name: &str, value: &str) -> Self {
        self.stylesheet = self.stylesheet.style_important(name, value);
        self
    }

    #[inline]
    pub fn style_signal<B, C>(mut self, name: &str, value: C) -> Self
        where B: AsOptionStr,
              C: IntoSignal<Item = B>,
              C::Signal: 'static {

        self.stylesheet = self.stylesheet.style_signal(name, value);
        self
    }

    #[inline]
    pub fn style_important_signal<B, C>(mut self, name: &str, value: C) -> Self
        where B: AsOptionStr,
              C: IntoSignal<Item = B>,
              C::Signal: 'static {

        self.stylesheet = self.stylesheet.style_important_signal(name, value);
        self
    }

    // TODO return a Handle ?
    #[inline]
    pub fn done(self) -> String {
        self.stylesheet.done();
        self.class_name
    }
}
