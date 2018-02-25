use stdweb::Reference;
use stdweb::web::TextNode;
use stdweb::traits::{IElement, IHtmlElement, INode};
use stdweb::unstable::TryInto;
use dom::{Dom, IStyle, Dynamic};
use callbacks::Callbacks;
use signals::Signal;
use operations;


pub trait IntoDynamic where Self: Sized {
    fn dynamic(self) -> Dynamic<Self>;
}

impl<A> IntoDynamic for A {
    #[inline]
    fn dynamic(self) -> Dynamic<Self> {
        Dynamic(self)
    }
}


pub trait Text {
    fn into_dom(self) -> Dom;
}

pub trait Property {
    fn set_property<A: AsRef<Reference> + Clone + 'static>(self, element: &A, callbacks: &mut Callbacks, name: &str);
}

pub trait Attribute {
    fn set_attribute<A: IElement + Clone + 'static>(self, element: &A, callbacks: &mut Callbacks, name: &str, namespace: Option<&str>);
}

pub trait Style {
    fn set_style<A: IStyle + Clone + 'static>(self, element: &A, callbacks: &mut Callbacks, name: &str, important: bool);
}

pub trait Class {
    fn toggle_class<A: IElement + Clone + 'static>(self, element: &A, callbacks: &mut Callbacks, name: &str);
}

pub trait Focused {
    fn toggle_focused<A: IHtmlElement + Clone + 'static>(self, element: &A, callbacks: &mut Callbacks);
}

pub trait Children {
    fn insert_children<A: INode + Clone + 'static>(self, element: &A, callbacks: &mut Callbacks);
}


impl<'a> Text for &'a str {
    #[inline]
    fn into_dom(self) -> Dom {
        Dom::new(js!( return document.createTextNode(@{self}); ).try_into().unwrap())
    }
}

impl<'a> Property for &'a str {
    #[inline]
    fn set_property<A: AsRef<Reference>>(self, element: &A, _callbacks: &mut Callbacks, name: &str) {
        operations::set_property_str(element, name, self)
    }
}

impl<'a> Attribute for &'a str {
    #[inline]
    fn set_attribute<A: IElement>(self, element: &A, _callbacks: &mut Callbacks, name: &str, namespace: Option<&str>) {
        operations::set_attribute_str(element, name, self, namespace)
    }
}

impl<'a> Style for &'a str {
    #[inline]
    fn set_style<A: IStyle>(self, element: &A, _callbacks: &mut Callbacks, name: &str, important: bool) {
        operations::set_style_str(element, name, self, important)
    }
}

impl Class for bool {
    #[inline]
    fn toggle_class<A: IElement>(self, element: &A, _callbacks: &mut Callbacks, name: &str) {
        operations::toggle_class_bool(element, name, self)
    }
}

impl Focused for bool {
    #[inline]
    fn toggle_focused<A: IHtmlElement + Clone + 'static>(self, element: &A, callbacks: &mut Callbacks) {
        operations::set_focused_bool(element, callbacks, self)
    }
}

// TODO figure out how to make this owned rather than &mut
impl<'a> Children for &'a mut [Dom] {
    #[inline]
    fn insert_children<A: INode>(self, element: &A, callbacks: &mut Callbacks) {
        operations::insert_children_slice(element, callbacks, self)
    }
}


impl<A: Signal<Item = String> + 'static> Text for Dynamic<A> {
    // TODO should this inline ?
    fn into_dom(self) -> Dom {
        let element: TextNode = js!( return document.createTextNode(""); ).try_into().unwrap();

        let mut callbacks = Callbacks::new();

        operations::set_text_signal(&element, &mut callbacks, self.0);

        Dom {
            element: element.into(),
            callbacks: callbacks,
        }
    }
}

impl<A: Signal<Item = Option<String>> + 'static> Property for Dynamic<A> {
    #[inline]
    fn set_property<B: AsRef<Reference> + Clone + 'static>(self, element: &B, callbacks: &mut Callbacks, name: &str) {
        operations::set_property_signal(element, callbacks, name, self.0)
    }
}

impl<A: Signal<Item = Option<String>> + 'static> Attribute for Dynamic<A> {
    #[inline]
    fn set_attribute<B: IElement + Clone + 'static>(self, element: &B, callbacks: &mut Callbacks, name: &str, namespace: Option<&str>) {
        operations::set_attribute_signal(element, callbacks, name, self.0, namespace)
    }
}

impl<A: Signal<Item = Option<String>> + 'static> Style for Dynamic<A> {
    #[inline]
    fn set_style<B: IStyle + Clone + 'static>(self, element: &B, callbacks: &mut Callbacks, name: &str, important: bool) {
        operations::set_style_signal(element, callbacks, name, self.0, important)
    }
}

impl<A: Signal<Item = bool> + 'static> Class for Dynamic<A> {
    #[inline]
    fn toggle_class<B: IElement + Clone + 'static>(self, element: &B, callbacks: &mut Callbacks, name: &str) {
        operations::toggle_class_signal(element, callbacks, name, self.0)
    }
}

impl<A: Signal<Item = bool> + 'static> Focused for Dynamic<A> {
    #[inline]
    fn toggle_focused<B: IHtmlElement + Clone + 'static>(self, element: &B, callbacks: &mut Callbacks) {
        operations::set_focused_signal(element, callbacks, self.0)
    }
}

impl<A, B> Children for Dynamic<B>
    where A: IntoIterator<Item = Dom>,
          B: Signal<Item = A> + 'static {
    #[inline]
    fn insert_children<C: INode + Clone + 'static>(self, element: &C, callbacks: &mut Callbacks) {
        operations::insert_children_signal(element, callbacks, self.0)
    }
}
