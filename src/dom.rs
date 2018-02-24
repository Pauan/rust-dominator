use std;
use std::sync::Mutex;
use stdweb::{Reference, Value, ReferenceType};
use stdweb::unstable::{TryFrom, TryInto};
use stdweb::web::{IEventTarget, INode, IElement, IHtmlElement, Node, TextNode};
use stdweb::web::event::ConcreteEvent;
use signal::Signal;


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


pub mod traits {
    use super::IStyle;
    use super::internal::Callbacks;
    use stdweb::Reference;
    use stdweb::web::{INode, IElement, IHtmlElement, TextNode};

    pub trait DomBuilder {
        type Value;

        fn value(&self) -> &Self::Value;

        fn callbacks(&mut self) -> &mut Callbacks;
    }

    pub trait DomText {
        fn set_text<A>(self, &mut A)
            // TODO use an interface rather than TextNode
            where A: DomBuilder<Value = TextNode>;
    }

    pub trait DomProperty {
        fn set_property<A, B>(self, &mut B, &str)
            // TODO it would be nice to be able to remove this Clone constraint somehow
            where A: AsRef<Reference> + Clone + 'static,
                  B: DomBuilder<Value = A>;
    }

    pub trait DomAttribute {
        fn set_attribute<A, B>(self, &mut B, &str, Option<&str>)
            // TODO it would be nice to be able to remove this Clone constraint somehow
            where A: IElement + Clone + 'static,
                  B: DomBuilder<Value = A>;
    }

    pub trait DomClass {
        fn toggle_class<A, B>(self, &mut B, &str)
            // TODO it would be nice to be able to remove this Clone constraint somehow
            where A: IElement + Clone + 'static,
                  B: DomBuilder<Value = A>;
    }

    pub trait DomStyle {
        fn set_style<A, B>(self, &mut B, &str, bool)
            // TODO it would be nice to be able to remove this Clone constraint somehow
            where A: IStyle + Clone + 'static,
                  B: DomBuilder<Value = A>;
    }

    pub trait DomFocused {
        fn set_focused<A, B>(self, &mut B)
            // TODO it would be nice to be able to remove this Clone constraint somehow
            where A: IHtmlElement + Clone + 'static,
                  B: DomBuilder<Value = A>;
    }

    pub trait DomChildren {
        fn insert_children<A, B>(self, &mut B)
            // TODO it would be nice to be able to remove this Clone constraint somehow
            where A: INode + Clone + 'static,
                  B: DomBuilder<Value = A>;
    }
}


pub mod dom_operations {
    use std;
    use stdweb::unstable::{TryFrom, TryInto};
    use stdweb::{Value, Reference};
    use stdweb::web::{TextNode, INode, IHtmlElement, IElement};


    #[inline]
    pub fn create_element_ns<A: IElement>(name: &str, namespace: &str) -> A
        where <A as TryFrom<Value>>::Error: std::fmt::Debug {
        js!( return document.createElementNS(@{namespace}, @{name}); ).try_into().unwrap()
    }

    // TODO this should be in stdweb
    #[inline]
    pub fn set_text(element: &TextNode, value: &str) {
        js! { @(no_return)
            // http://jsperf.com/textnode-performance
            @{element}.data = @{value};
        }
    }

    // TODO replace with element.focus() and element.blur()
    // TODO make element.focus() and element.blur() inline
    #[inline]
    pub fn set_focused<A: IHtmlElement>(element: &A, focused: bool) {
        js! { @(no_return)
            var element = @{element.as_ref()};

            if (@{focused}) {
                element.focus();

            } else {
                element.blur();
            }
        }
    }

    #[inline]
    pub fn toggle_class<A: IElement>(element: &A, name: &str, toggle: bool) {
        js! { @(no_return)
            @{element.as_ref()}.classList.toggle(@{name}, @{toggle});
        }
    }


    #[inline]
    fn _set_attribute_ns<A: IElement>(element: &A, name: &str, value: &str, namespace: &str) {
        js! { @(no_return)
            @{element.as_ref()}.setAttributeNS(@{namespace}, @{name}, @{value});
        }
    }

    #[inline]
    fn _set_attribute<A: IElement>(element: &A, name: &str, value: &str) {
        js! { @(no_return)
            @{element.as_ref()}.setAttribute(@{name}, @{value});
        }
    }

    // TODO check that the attribute *actually* was changed
    #[inline]
    pub fn set_attribute<A: IElement>(element: &A, name: &str, value: &str, namespace: Option<&str>) {
        match namespace {
            Some(namespace) => _set_attribute_ns(element, name, value, namespace),
            None => _set_attribute(element, name, value),
        }
    }


    #[inline]
    fn _remove_attribute_ns<A: IElement>(element: &A, name: &str, namespace: &str) {
        js! { @(no_return)
            @{element.as_ref()}.removeAttributeNS(@{namespace}, @{name});
        }
    }

    #[inline]
    fn _remove_attribute<A: IElement>(element: &A, name: &str) {
        js! { @(no_return)
            @{element.as_ref()}.removeAttribute(@{name});
        }
    }

    #[inline]
    pub fn remove_attribute<A: IElement>(element: &A, name: &str, namespace: Option<&str>) {
        match namespace {
            Some(namespace) => _remove_attribute_ns(element, name, namespace),
            None => _remove_attribute(element, name),
        }
    }


    // TODO check that the property *actually* was changed ?
    #[inline]
    pub fn set_property<A: AsRef<Reference>>(obj: &A, name: &str, value: &str) {
        js! { @(no_return)
            @{obj.as_ref()}[@{name}] = @{value};
        }
    }


    // TODO make this work on Nodes, not just Elements
    // TODO is this the most efficient way to remove all children ?
    #[inline]
    pub fn remove_all_children<A: INode>(element: &A) {
        js! { @(no_return)
            @{element.as_ref()}.innerHTML = "";
        }
    }
}


pub mod internal {
    use std;


    // TODO replace this with FnOnce later
    trait IRemoveCallback {
        fn call(self: Box<Self>);
    }

    impl<F: FnOnce()> IRemoveCallback for F {
        #[inline]
        fn call(self: Box<Self>) {
            self();
        }
    }


    // TODO replace this with FnOnce later
    trait IInsertCallback {
        fn call(self: Box<Self>, &mut Callbacks);
    }

    impl<F: FnOnce(&mut Callbacks)> IInsertCallback for F {
        #[inline]
        fn call(self: Box<Self>, callbacks: &mut Callbacks) {
            self(callbacks);
        }
    }


    pub struct InsertCallback(Box<IInsertCallback>);

    pub struct RemoveCallback(Box<IRemoveCallback>);

    impl std::fmt::Debug for InsertCallback {
        fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "InsertCallback")
        }
    }

    impl std::fmt::Debug for RemoveCallback {
        fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "RemoveCallback")
        }
    }


    #[derive(Debug)]
    pub struct Callbacks {
        pub after_insert: Vec<InsertCallback>,
        pub after_remove: Vec<RemoveCallback>,
        // TODO figure out a better way
        pub(crate) trigger_remove: bool,
    }

    impl Callbacks {
        #[inline]
        pub fn new() -> Self {
            Self {
                after_insert: vec![],
                after_remove: vec![],
                trigger_remove: true,
            }
        }

        #[inline]
        pub fn after_insert<A: FnOnce(&mut Callbacks) + 'static>(&mut self, callback: A) {
            self.after_insert.push(InsertCallback(Box::new(callback)));
        }

        #[inline]
        pub fn after_remove<A: FnOnce() + 'static>(&mut self, callback: A) {
            self.after_remove.push(RemoveCallback(Box::new(callback)));
        }

        // TODO runtime checks to make sure this isn't called multiple times ?
        #[inline]
        pub fn trigger_after_insert(&mut self) {
            let mut callbacks = Callbacks::new();

            // TODO verify that this is correct
            // TODO is this the most efficient way to accomplish this ?
            std::mem::swap(&mut callbacks.after_remove, &mut self.after_remove);

            for f in self.after_insert.drain(..) {
                f.0.call(&mut callbacks);
            }

            self.after_insert.shrink_to_fit();

            // TODO figure out a better way of verifying this
            assert_eq!(callbacks.after_insert.len(), 0);

            // TODO verify that this is correct
            std::mem::swap(&mut callbacks.after_remove, &mut self.after_remove);
        }

        #[inline]
        fn trigger_after_remove(&mut self) {
            for f in self.after_remove.drain(..) {
                f.0.call();
            }

            // TODO is this a good idea?
            self.after_remove.shrink_to_fit();
        }
    }

    impl Drop for Callbacks {
        #[inline]
        fn drop(&mut self) {
            if self.trigger_remove {
                self.trigger_after_remove();
            }
        }
    }
}


use self::traits::{DomBuilder, DomText, DomProperty, DomAttribute, DomClass, DomStyle, DomFocused, DomChildren};
use self::internal::Callbacks;


// https://developer.mozilla.org/en-US/docs/Web/API/Document/createElementNS#Valid%20Namespace%20URIs
pub const HTML_NAMESPACE: &str = "http://www.w3.org/1999/xhtml";
pub const SVG_NAMESPACE: &str = "http://www.w3.org/2000/svg";


pub struct TextBuilder {
    element: TextNode,
    callbacks: Callbacks,
}

impl DomBuilder for TextBuilder {
    type Value = TextNode;

    #[inline]
    fn value(&self) -> &Self::Value {
        &self.element
    }

    #[inline]
    fn callbacks(&mut self) -> &mut Callbacks {
        &mut self.callbacks
    }
}


#[derive(Debug)]
pub struct Dom {
    element: Node,
    callbacks: Callbacks,
}

impl Dom {
    #[inline]
    fn new(element: Node) -> Self {
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

        dom.callbacks.after_remove(move || drop(state));

        dom
    }

    // TODO return a Handle
    #[inline]
    pub fn insert_into<A: INode>(mut self, parent: &A) {
        parent.append_child(&self.element);

        self.callbacks.trigger_after_insert();

        // This prevents it from calling trigger_after_remove
        self.callbacks.trigger_remove = false;
    }
}

impl<A: DomText> From<A> for Dom {
    #[inline]
    fn from(dom: A) -> Self {
        let mut text = TextBuilder {
            element: js!( return document.createTextNode(""); ).try_into().unwrap(),
            callbacks: Callbacks::new(),
        };

        dom.set_text(&mut text);

        Self {
            element: text.element.into(),
            callbacks: text.callbacks,
        }
    }
}

impl<'a> From<&'a str> for Dom {
    #[inline]
    fn from(value: &'a str) -> Self {
        Self::new(js!( return document.createTextNode(@{value}); ).try_into().unwrap())
    }
}


pub struct HtmlBuilder<A> {
    element: A,
    callbacks: Callbacks,
    // TODO verify this with static types instead ?
    has_children: bool,
}

impl<A> DomBuilder for HtmlBuilder<A> {
    type Value = A;

    #[inline]
    fn value(&self) -> &Self::Value {
        &self.element
    }

    #[inline]
    fn callbacks(&mut self) -> &mut Callbacks {
        &mut self.callbacks
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
    pub fn property<B: DomProperty>(mut self, name: &str, value: B) -> Self {
        value.set_property(&mut self, name);
        self
    }
}

impl<A: IEventTarget> HtmlBuilder<A> {
    // TODO maybe inline this ?
    // TODO replace with element.add_event_listener
    fn _event<T, F>(&mut self, listener: F)
        where T: ConcreteEvent,
              F: FnMut(T) + 'static {

        let element = self.element.as_ref();

        let listener = js!(
            var listener = @{listener};
            @{&element}.addEventListener(@{T::EVENT_TYPE}, listener);
            return listener;
        );

        let element = element.clone();

        self.callbacks.after_remove(move || {
            js! { @(no_return)
                var listener = @{listener};
                @{element}.removeEventListener(@{T::EVENT_TYPE}, listener);
                listener.drop();
            }
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
    pub fn children<B: DomChildren>(mut self, children: B) -> Self {
        assert_eq!(self.has_children, false);
        self.has_children = true;

        children.insert_children(&mut self);
        self
    }
}

impl<A: IElement + Clone + 'static> HtmlBuilder<A> {
    #[inline]
    pub fn attribute<B: DomAttribute>(mut self, name: &str, value: B) -> Self {
        value.set_attribute(&mut self, name, None);
        self
    }

    #[inline]
    pub fn attribute_namespace<B: DomAttribute>(mut self, name: &str, value: B, namespace: &str) -> Self {
        value.set_attribute(&mut self, name, Some(namespace));
        self
    }

    #[inline]
    pub fn class<B: DomClass>(mut self, name: &str, value: B) -> Self {
        value.toggle_class(&mut self, name);
        self
    }
}

impl<A: IHtmlElement + Clone + 'static> HtmlBuilder<A> {
    #[inline]
    pub fn style<B: DomStyle>(mut self, name: &str, value: B) -> Self {
        value.set_style(&mut self, name, false);
        self
    }

    #[inline]
    pub fn style_important<B: DomStyle>(mut self, name: &str, value: B) -> Self {
        value.set_style(&mut self, name, true);
        self
    }

    #[inline]
    pub fn focused<B: DomFocused>(mut self, value: B) -> Self {
        value.set_focused(&mut self);
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


impl<'a> DomProperty for &'a str {
    #[inline]
    fn set_property<A: AsRef<Reference>, B: DomBuilder<Value = A>>(self, builder: &mut B, name: &str) {
        dom_operations::set_property(builder.value(), name, self);
    }
}

impl<'a> DomAttribute for &'a str {
    #[inline]
    fn set_attribute<A: IElement, B: DomBuilder<Value = A>>(self, builder: &mut B, name: &str, namespace: Option<&str>) {
        dom_operations::set_attribute(builder.value(), name, self, namespace);
    }
}

impl DomClass for bool {
    #[inline]
    fn toggle_class<A: IElement, B: DomBuilder<Value = A>>(self, builder: &mut B, name: &str) {
        dom_operations::toggle_class(builder.value(), name, self);
    }
}

impl<'a> DomStyle for &'a str {
    #[inline]
    fn set_style<A: IStyle, B: DomBuilder<Value = A>>(self, builder: &mut B, name: &str, important: bool) {
        builder.value().set_style(name, self, important);
    }
}

impl DomFocused for bool {
    #[inline]
    fn set_focused<A: IHtmlElement + Clone + 'static, B: DomBuilder<Value = A>>(self, builder: &mut B) {
        let value = builder.value().clone();

        // This needs to use `after_insert` because calling `.focus()` on an element before it is in the DOM has no effect
        builder.callbacks().after_insert(move |_| {
            dom_operations::set_focused(&value, self);
        });
    }
}

// TODO figure out how to make this owned rather than &mut
// TODO impl<'a, A: IntoIterator<Item = &'a mut Dom>> DomChildren for A {
impl<'a> DomChildren for &'a mut [Dom] {
    #[inline]
    fn insert_children<B: INode, C: DomBuilder<Value = B>>(self, builder: &mut C) {
        for dom in self.into_iter() {
            {
                let callbacks = builder.callbacks();
                callbacks.after_insert.append(&mut dom.callbacks.after_insert);
                callbacks.after_remove.append(&mut dom.callbacks.after_remove);
            }

            builder.value().append_child(&dom.element);
        }
    }
}


impl<S: Signal<Item = String> + 'static> DomProperty for S {
    // TODO inline this ?
    fn set_property<A: AsRef<Reference> + Clone + 'static, B: DomBuilder<Value = A>>(self, builder: &mut B, name: &str) {
        let element = builder.value().clone();
        let name = name.to_owned();

        let handle = self.for_each(move |value| {
            dom_operations::set_property(&element, &name, &value);
        });

        builder.callbacks().after_remove(move || handle.stop());
    }
}

impl<S: Signal<Item = Option<String>> + 'static> DomAttribute for S {
    // TODO inline this ?
    fn set_attribute<A: IElement + Clone + 'static, B: DomBuilder<Value = A>>(self, builder: &mut B, name: &str, namespace: Option<&str>) {
        let element = builder.value().clone();
        let name = name.to_owned();
        let namespace = namespace.map(|x| x.to_owned());

        let handle = self.for_each(move |value| {
            // TODO figure out a way to avoid this
            let namespace = namespace.as_ref().map(|x| x.as_str());

            match value {
                Some(value) => dom_operations::set_attribute(&element, &name, &value, namespace),
                None => dom_operations::remove_attribute(&element, &name, namespace),
            }
        });

        builder.callbacks().after_remove(move || handle.stop());
    }
}

impl<S: Signal<Item = bool> + 'static> DomClass for S {
    // TODO inline this ?
    fn toggle_class<A: IElement + Clone + 'static, B: DomBuilder<Value = A>>(self, builder: &mut B, name: &str) {
        let element = builder.value().clone();
        let name = name.to_owned();

        let handle = self.for_each(move |value| {
            dom_operations::toggle_class(&element, &name, value);
        });

        builder.callbacks().after_remove(move || handle.stop());
    }
}

impl<S: Signal<Item = Option<String>> + 'static> DomStyle for S {
    // TODO inline this ?
    fn set_style<A: IStyle + Clone + 'static, B: DomBuilder<Value = A>>(self, builder: &mut B, name: &str, important: bool) {
        let element = builder.value().clone();
        let name = name.to_owned();

        let handle = self.for_each(move |value| {
            match value {
                Some(value) => element.set_style(&name, &value, important),
                None => element.set_style(&name, "", important),
            }
        });

        builder.callbacks().after_remove(move || handle.stop());
    }
}

impl<S: Signal<Item = bool> + 'static> DomFocused for S {
    // TODO inline this ?
    fn set_focused<A: IHtmlElement + Clone + 'static, B: DomBuilder<Value = A>>(self, builder: &mut B) {
        let element = builder.value().clone();
        let callbacks = builder.callbacks();

        // This needs to use `after_insert` because calling `.focus()` on an element before it is in the DOM has no effect
        callbacks.after_insert(move |callbacks| {
            let handle = self.for_each(move |value| {
                dom_operations::set_focused(&element, value);
            });

            // TODO verify that this is correct under all circumstances
            callbacks.after_remove(move || handle.stop());
        });
    }
}

impl<A: IntoIterator<Item = Dom>, S: Signal<Item = A> + 'static> DomChildren for S {
    // TODO inline this ?
    fn insert_children<B: INode + Clone + 'static, C: DomBuilder<Value = B>>(self, builder: &mut C) {
        let element = builder.value().clone();

        let mut old_children: Vec<Dom> = vec![];

        let handle = self.for_each(move |value| {
            dom_operations::remove_all_children(&element);

            old_children = value.into_iter().map(|mut dom| {
                element.append_child(&dom.element);

                // TODO don't trigger this if the parent isn't inserted into the DOM
                dom.callbacks.trigger_after_insert();

                dom
            }).collect();
        });

        builder.callbacks().after_remove(move || handle.stop());
    }
}


pub struct StylesheetBuilder {
    element: CssStyleRule,
    callbacks: Callbacks,
}

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
    pub fn style<B: DomStyle>(mut self, name: &str, value: B) -> Self {
        value.set_style(&mut self, name, false);
        self
    }

    #[inline]
    pub fn style_important<B: DomStyle>(mut self, name: &str, value: B) -> Self {
        value.set_style(&mut self, name, true);
        self
    }

    // TODO return a Handle
    #[inline]
    pub fn done(mut self) {
        self.callbacks.trigger_after_insert();

        // This prevents it from calling trigger_after_remove
        self.callbacks.trigger_remove = false;
    }
}

impl DomBuilder for StylesheetBuilder {
    type Value = CssStyleRule;

    #[inline]
    fn value(&self) -> &Self::Value {
        &self.element
    }

    #[inline]
    fn callbacks(&mut self) -> &mut Callbacks {
        &mut self.callbacks
    }
}


pub struct ClassBuilder {
    stylesheet: StylesheetBuilder,
    class_name: String,
}

impl ClassBuilder {
    #[inline]
    pub fn new() -> Self {
        let class_name = {
            lazy_static! {
                // TODO can this be made more efficient ?
                static ref CLASS_ID: Mutex<u32> = Mutex::new(0);
            }

            let mut id = CLASS_ID.lock().unwrap();

            *id += 1;

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
    pub fn style<B: DomStyle>(mut self, name: &str, value: B) -> Self {
        self.stylesheet = self.stylesheet.style(name, value);
        self
    }

    #[inline]
    pub fn style_important<B: DomStyle>(mut self, name: &str, value: B) -> Self {
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

impl DomBuilder for ClassBuilder {
    type Value = CssStyleRule;

    #[inline]
    fn value(&self) -> &Self::Value {
        self.stylesheet.value()
    }

    #[inline]
    fn callbacks(&mut self) -> &mut Callbacks {
        self.stylesheet.callbacks()
    }
}


#[macro_export]
macro_rules! html {
    ($kind:expr => $t:ty) => {
        html!($kind => $t, {})
    };
    ($kind:expr => $t:ty, { $( $name:ident( $( $args:expr ),* ); )* }) => {{
        let a: $crate::HtmlBuilder<$t> = $crate::HtmlBuilder::new($kind)$(.$name($($args),*))*;
        let b: $crate::Dom = a.into();
        b
    }};

    ($kind:expr) => {
        // TODO need better hygiene for HtmlElement
        html!($kind => HtmlElement)
    };
    ($kind:expr, { $( $name:ident( $( $args:expr ),* ); )* }) => {{
        // TODO need better hygiene for HtmlElement
        html!($kind => HtmlElement, { $( $name( $( $args ),* ); )* })
    }};
}


#[macro_export]
macro_rules! stylesheet {
    ($rule:expr) => {
        stylesheet!($rule, {})
    };
    ($rule:expr, { $( $name:ident( $( $args:expr ),* ); )* }) => {{
        $crate::StylesheetBuilder::new($rule)$(.$name($($args),*))*.done()
    }};
}


#[macro_export]
macro_rules! class {
    ($( $name:ident( $( $args:expr ),* ); )*) => {{
        $crate::ClassBuilder::new()$(.$name($($args),*))*.done()
    }};
}


// TODO move into stdweb
#[macro_export]
macro_rules! clone {
    ({$($x:ident),+} $y:expr) => {{
        $(let $x = $x.clone();)+
        $y
    }};
}
