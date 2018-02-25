use stdweb::PromiseFuture;
use discard::{Discard, DiscardOnDrop};
use stdweb::Reference;
use stdweb::web::TextNode;
use signals::{Signal, cancelable_future, CancelableFutureHandle};
use dom_operations;
use dom::{Dom, IStyle};
use callbacks::Callbacks;
use std::iter::IntoIterator;
use stdweb::traits::{INode, IElement, IHtmlElement};


#[inline]
fn for_each<A, B>(signal: A, mut callback: B) -> CancelableFutureHandle
    where A: Signal + 'static,
          B: FnMut(A::Item) + 'static {

    let future = signal.for_each(move |value| {
        callback(value);
        Ok(())
    });

    // TODO make this more efficient ?
    let (handle, future) = cancelable_future(future, |_| ());

    PromiseFuture::spawn(future);

    DiscardOnDrop::leak(handle)
}


// TODO inline this ?
pub fn set_text_signal<A>(element: &TextNode, callbacks: &mut Callbacks, signal: A)
    where A: Signal<Item = String> + 'static {

    let element = element.clone();

    let handle = for_each(signal, move |value| {
        dom_operations::set_text(&element, &value);
    });

    callbacks.after_remove(move || handle.discard());
}


// TODO inline this ?
pub fn set_property_signal<'a, A, B>(element: &A, callbacks: &mut Callbacks, name: &str, signal: B)
    where A: AsRef<Reference> + Clone + 'static,
          B: Signal<Item = Option<String>> + 'static {

    let element = element.clone();
    let name = name.to_owned();

    let handle = for_each(signal, move |value| {
        dom_operations::set_property(&element, &name, value.as_ref().map(|x| x.as_str()));
    });

    callbacks.after_remove(move || handle.discard());
}

#[inline]
pub fn set_property_str<A: AsRef<Reference>>(element: &A, name: &str, value: &str) {
    dom_operations::set_property(element, name, Some(value))
}


// TODO inline this ?
pub fn set_attribute_signal<A, B>(element: &A, callbacks: &mut Callbacks, name: &str, signal: B, namespace: Option<&str>)
    where A: IElement + Clone + 'static,
          B: Signal<Item = Option<String>> + 'static {

    let element = element.clone();
    let name = name.to_owned();
    let namespace = namespace.map(|x| x.to_owned());

    let handle = for_each(signal, move |value| {
        // TODO figure out a way to avoid this
        let namespace = namespace.as_ref().map(|x| x.as_str());

        match value {
            Some(value) => dom_operations::set_attribute(&element, &name, &value, namespace),
            None => dom_operations::remove_attribute(&element, &name, namespace),
        }
    });

    callbacks.after_remove(move || handle.discard());
}

#[inline]
pub fn set_attribute_str<A: IElement>(element: &A, name: &str, value: &str, namespace: Option<&str>) {
    dom_operations::set_attribute(element, name, value, namespace)
}


// TODO inline this ?
pub fn toggle_class_signal<A, B>(element: &A, callbacks: &mut Callbacks, name: &str, signal: B)
    where A: IElement + Clone + 'static,
          B: Signal<Item = bool> + 'static {

    let element = element.clone();
    let name = name.to_owned();

    let handle = for_each(signal, move |value| {
        dom_operations::toggle_class(&element, &name, value);
    });

    callbacks.after_remove(move || handle.discard());
}

#[inline]
pub fn toggle_class_bool<A: IElement>(element: &A, name: &str, value: bool) {
    dom_operations::toggle_class(element, name, value)
}


// TODO inline this ?
pub fn set_style_signal<A, B>(element: &A, callbacks: &mut Callbacks, name: &str, signal: B, important: bool)
    where A: IStyle + Clone + 'static,
          B: Signal<Item = Option<String>> + 'static {

    let element = element.clone();
    let name = name.to_owned();

    let handle = for_each(signal, move |value| {
        match value {
            Some(value) => element.set_style(&name, &value, important),
            None => element.set_style(&name, "", important),
        }
    });

    callbacks.after_remove(move || handle.discard());
}

#[inline]
pub fn set_style_str<A: IStyle>(element: &A, name: &str, value: &str, important: bool) {
    element.set_style(name, value, important)
}


// TODO inline this ?
pub fn set_focused_signal<A, B>(element: &A, callbacks: &mut Callbacks, signal: B)
    where A: IHtmlElement + Clone + 'static,
          B: Signal<Item = bool> + 'static {

    let element = element.clone();

    // This needs to use `after_insert` because calling `.focus()` on an element before it is in the DOM has no effect
    callbacks.after_insert(move |callbacks| {
        let handle = for_each(signal, move |value| {
            dom_operations::set_focused(&element, value);
        });

        // TODO verify that this is correct under all circumstances
        callbacks.after_remove(move || handle.discard());
    });
}

#[inline]
pub fn set_focused_bool<A: IHtmlElement + Clone + 'static>(element: &A, callbacks: &mut Callbacks, value: bool) {
    let element = element.clone();

    // This needs to use `after_insert` because calling `.focus()` on an element before it is in the DOM has no effect
    callbacks.after_insert(move |_| {
        dom_operations::set_focused(&element, value);
    });
}


// TODO inline this ?
pub fn insert_children_signal<A, B, C>(element: &A, callbacks: &mut Callbacks, signal: C)
    where A: INode + Clone + 'static,
          B: IntoIterator<Item = Dom>,
          C: Signal<Item = B> + 'static {

    let element = element.clone();

    let mut old_children: Vec<Dom> = vec![];

    let handle = for_each(signal, move |value| {
        dom_operations::remove_all_children(&element);

        old_children = value.into_iter().map(|mut dom| {
            element.append_child(&dom.element);

            // TODO don't trigger this if the parent isn't inserted into the DOM
            dom.callbacks.trigger_after_insert();

            dom
        }).collect();
    });

    // TODO verify that this will drop `old_children`
    callbacks.after_remove(move || handle.discard());
}

#[inline]
pub fn insert_children_slice<A: INode>(element: &A, callbacks: &mut Callbacks, value: &mut [Dom]) {
    for dom in value.into_iter() {
        callbacks.after_insert.append(&mut dom.callbacks.after_insert);
        callbacks.after_remove.append(&mut dom.callbacks.after_remove);

        element.append_child(&dom.element);
    }
}
