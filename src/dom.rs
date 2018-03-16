use std;
use stdweb::{Reference, Value, ReferenceType};
use stdweb::unstable::{TryFrom, TryInto};
use stdweb::web::{IEventTarget, INode, IElement, IHtmlElement, Node};
use stdweb::web::event::ConcreteEvent;
use callbacks::Callbacks;
use traits::*;
use dom_operations;
use operations::{BoxDiscard, spawn_future};
use futures::future::Future;
use discard::{Discard, DiscardOnDrop};


pub struct Dynamic<A>(pub(crate) A);


// TODO this should be in stdweb
pub trait IStyle: ReferenceType {
    // TODO check that the style *actually* was changed
    // TODO handle browser prefixes
    #[inline]
    fn set_style(&self, name: &str, value: &str, important: bool) {
        let important = if important { "important" } else { "" };

        js! { @(no_return)
            @{self.as_ref()}.style.setProperty(@{name}, @{value}, @{important});
        }
    }
}

impl<A: IHtmlElement> IStyle for A {}


// TODO this should be in stdweb
#[derive(Clone, Debug, PartialEq, Eq, ReferenceType)]
#[reference(instance_of = "CSSStyleRule")]
pub struct CssStyleRule(Reference);

impl IStyle for CssStyleRule {}


// https://developer.mozilla.org/en-US/docs/Web/API/Document/createElementNS#Valid%20Namespace%20URIs
pub const HTML_NAMESPACE: &str = "http://www.w3.org/1999/xhtml";
pub const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";


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
pub fn text<A: Text>(value: A) -> Dom {
    value.into_dom()
}


#[derive(Debug)]
pub struct Dom {
    pub(crate) element: Node,
    pub(crate) callbacks: Callbacks,
}

impl Dom {
    #[inline]
    pub(crate) fn new(element: Node) -> Self {
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

        // TODO make this more efficient somehow ? what if the value is already on the heap ?
        dom.callbacks.after_remove(BoxDiscard::new(state));

        dom
    }
}


pub struct HtmlBuilder<A> {
    element: A,
    callbacks: Callbacks,
    // TODO verify this with static types instead ?
    has_children: bool,
}

impl<A> HtmlBuilder<A> {
    #[inline]
    pub fn future<F>(mut self, future: F) -> Self where F: Future<Item = (), Error = ()> + 'static {
        self.callbacks.after_remove(DiscardOnDrop::leak(spawn_future(future)));
        self
    }
}

// TODO add in SVG nodes
impl<A: IElement> HtmlBuilder<A>
    where <A as TryFrom<Value>>::Error: std::fmt::Debug {

    #[inline]
    pub fn new(name: &str) -> Self {
        Self {
            element: dom_operations::create_element_ns(name, HTML_NAMESPACE),
            callbacks: Callbacks::new(),
            has_children: false,
        }
    }
}

impl<A: AsRef<Reference> + Clone + 'static> HtmlBuilder<A> {
    #[inline]
    pub fn property<B: Property>(mut self, name: &str, value: B) -> Self {
        value.set_property(&self.element, &mut self.callbacks, name);
        self
    }
}

struct EventListenerHandle {
    event: &'static str,
    element: Reference,
    listener: Value,
}

impl Discard for EventListenerHandle {
    #[inline]
    fn discard(self) {
        js! { @(no_return)
            var listener = @{&self.listener};
            @{&self.element}.removeEventListener(@{self.event}, listener);
            listener.drop();
        }
    }
}

impl<A: IEventTarget> HtmlBuilder<A> {
    // TODO maybe inline this ?
    // TODO replace with element.add_event_listener
    fn _event<T, F>(&mut self, listener: F)
        where T: ConcreteEvent,
              F: FnMut(T) + 'static {

        let element = self.element.as_ref().clone();

        let listener = js!(
            var listener = @{listener};
            @{&element}.addEventListener(@{T::EVENT_TYPE}, listener);
            return listener;
        );

        self.callbacks.after_remove(EventListenerHandle {
            event: T::EVENT_TYPE,
            element,
            listener,
        });
    }

    #[inline]
    pub fn event<T, F>(mut self, listener: F) -> Self
        where T: ConcreteEvent,
              F: FnMut(T) + 'static {
        self._event(listener);
        self
    }
}

impl<A: INode + Clone + 'static> HtmlBuilder<A> {
    #[inline]
    pub fn children<B: Children>(mut self, children: B) -> Self {
        assert_eq!(self.has_children, false);
        self.has_children = true;

        children.insert_children(&self.element, &mut self.callbacks);
        self
    }
}

impl<A: IElement + Clone + 'static> HtmlBuilder<A> {
    #[inline]
    pub fn attribute<B: Attribute>(mut self, name: &str, value: B) -> Self {
        value.set_attribute(&self.element, &mut self.callbacks, name, None);
        self
    }

    #[inline]
    pub fn attribute_namespace<B: Attribute>(mut self, name: &str, value: B, namespace: &str) -> Self {
        value.set_attribute(&self.element, &mut self.callbacks, name, Some(namespace));
        self
    }

    #[inline]
    pub fn class<B: Class>(mut self, name: &str, value: B) -> Self {
        value.toggle_class(&self.element, &mut self.callbacks, name);
        self
    }
}

impl<A: IHtmlElement + Clone + 'static> HtmlBuilder<A> {
    #[inline]
    pub fn style<B: Style>(mut self, name: &str, value: B) -> Self {
        value.set_style(&self.element, &mut self.callbacks, name, false);
        self
    }

    #[inline]
    pub fn style_important<B: Style>(mut self, name: &str, value: B) -> Self {
        value.set_style(&self.element, &mut self.callbacks, name, true);
        self
    }

    #[inline]
    pub fn focused<B: Focused>(mut self, value: B) -> Self {
        value.toggle_focused(&self.element, &mut self.callbacks);
        self
    }
}

impl<A: Into<Node>> From<HtmlBuilder<A>> for Dom {
    #[inline]
    fn from(dom: HtmlBuilder<A>) -> Self {
        Self {
            element: dom.element.into(),
            callbacks: dom.callbacks,
        }
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
    pub fn style<B: Style>(mut self, name: &str, value: B) -> Self {
        value.set_style(&self.element, &mut self.callbacks, name, false);
        self
    }

    #[inline]
    pub fn style_important<B: Style>(mut self, name: &str, value: B) -> Self {
        value.set_style(&self.element, &mut self.callbacks, name, true);
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
    pub fn style<B: Style>(mut self, name: &str, value: B) -> Self {
        self.stylesheet = self.stylesheet.style(name, value);
        self
    }

    #[inline]
    pub fn style_important<B: Style>(mut self, name: &str, value: B) -> Self {
        self.stylesheet = self.stylesheet.style_important(name, value);
        self
    }

    // TODO return a Handle ?
    #[inline]
    pub fn done(self) -> String {
        self.stylesheet.done();
        self.class_name
    }
}
