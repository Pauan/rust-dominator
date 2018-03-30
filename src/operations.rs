use std::rc::Rc;
use std::cell::{Cell, RefCell};
use stdweb::PromiseFuture;
use discard::{Discard, DiscardOnDrop};
use stdweb::{Reference, JsSerialize};
use stdweb::web::TextNode;
use futures_signals::signal::{Signal, cancelable_future, CancelableFutureHandle};
use futures_signals::signal_vec::{VecChange, SignalVec};
use dom_operations;
use dom::{Dom, IStyle};
use callbacks::Callbacks;
use std::iter::IntoIterator;
use stdweb::traits::{INode, IElement, IHtmlElement};
use futures::future::Future;


// TODO this should probably be in stdweb
#[inline]
pub fn spawn_future<F>(future: F) -> DiscardOnDrop<CancelableFutureHandle>
    where F: Future<Item = (), Error = ()> + 'static {
    // TODO make this more efficient ?
    let (handle, future) = cancelable_future(future, |_| ());

    PromiseFuture::spawn(future);

    handle
}


#[inline]
fn for_each<A, B>(signal: A, mut callback: B) -> CancelableFutureHandle
    where A: Signal + 'static,
          B: FnMut(A::Item) + 'static {

    DiscardOnDrop::leak(spawn_future(signal.for_each(move |value| {
        callback(value);
        Ok(())
    })))
}


#[inline]
fn for_each_vec<A, B>(signal: A, mut callback: B) -> CancelableFutureHandle
    where A: SignalVec + 'static,
          B: FnMut(VecChange<A::Item>) + 'static {

    DiscardOnDrop::leak(spawn_future(signal.for_each(move |value| {
        callback(value);
        Ok(())
    })))
}


// TODO inline this ?
pub fn set_text_signal<A>(element: &TextNode, callbacks: &mut Callbacks, signal: A)
    where A: Signal<Item = String> + 'static {

    let element = element.clone();

    let handle = for_each(signal, move |value| {
        dom_operations::set_text(&element, &value);
    });

    callbacks.after_remove(handle);
}


// TODO inline this ?
pub fn set_property_signal<'a, A, B, C>(element: &A, callbacks: &mut Callbacks, name: &str, signal: C)
    where A: AsRef<Reference> + Clone + 'static,
          B: JsSerialize,
          C: Signal<Item = B> + 'static {

    let element = element.clone();
    let name = name.to_owned();

    let handle = for_each(signal, move |value| {
        dom_operations::set_property(&element, &name, &value);
    });

    callbacks.after_remove(handle);
}

#[inline]
pub fn set_property_str<A: AsRef<Reference>, B: JsSerialize>(element: &A, name: &str, value: &B) {
    dom_operations::set_property(element, name, value)
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

    callbacks.after_remove(handle);
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

    callbacks.after_remove(handle);
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

    callbacks.after_remove(handle);
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
        callbacks.after_remove(handle);
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


/*
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
    callbacks.after_remove(handle);
}*/

#[inline]
pub fn insert_children_iter<'a, A: INode, B: IntoIterator<Item = &'a mut Dom>>(element: &A, callbacks: &mut Callbacks, value: B) {
    for dom in value.into_iter() {
        callbacks.after_insert.append(&mut dom.callbacks.after_insert);
        callbacks.after_remove.append(&mut dom.callbacks.after_remove);

        element.append_child(&dom.element);
    }
}


// TODO move this into the discard crate
// TODO verify that this is correct and doesn't leak memory or cause memory safety
pub struct RcDiscard<A>(*const A);

impl<A> RcDiscard<A> {
    #[inline]
    pub fn new(value: Rc<A>) -> Self {
        RcDiscard(Rc::into_raw(value))
    }
}

impl<A> Discard for RcDiscard<A> {
    #[inline]
    fn discard(self) {
        unsafe {
            Rc::from_raw(self.0);
        }
    }
}


// TODO move this into the discard crate
// TODO verify that this is correct and doesn't leak memory or cause memory safety
pub struct BoxDiscard<A>(*mut A);

impl<A> BoxDiscard<A> {
    #[inline]
    pub fn new(value: A) -> Self {
        Self::from_box(Box::new(value))
    }

    #[inline]
    pub fn from_box(value: Box<A>) -> Self {
        BoxDiscard(Box::into_raw(value))
    }
}

impl<A> Discard for BoxDiscard<A> {
    #[inline]
    fn discard(self) {
        unsafe {
            Box::from_raw(self.0);
        }
    }
}


pub fn insert_children_signal_vec<A, B>(element: &A, callbacks: &mut Callbacks, signal: B)
    where A: INode + Clone + 'static,
          B: SignalVec<Item = Dom> + 'static {

    let element = element.clone();

    // TODO does this create a new struct type every time ?
    struct State {
        is_inserted: Cell<bool>,
        children: RefCell<Vec<Dom>>,
    }

    // TODO use two separate Rcs ?
    let state = Rc::new(State {
        is_inserted: Cell::new(false),
        children: RefCell::new(vec![]),
    });

    {
        let state = state.clone();

        callbacks.after_insert(move |_| {
            if !state.is_inserted.replace(true) {
                let mut children = state.children.borrow_mut();

                for dom in children.iter_mut() {
                    dom.callbacks.trigger_after_insert();
                }
            }
        });
    }

    let handle = for_each_vec(signal, move |change| {
        match change {
            VecChange::Replace { values } => {
                dom_operations::remove_all_children(&element);

                let mut children = state.children.borrow_mut();

                for dom in children.drain(..) {
                    dom.callbacks.discard();
                }

                *children = values;

                let is_inserted = state.is_inserted.get();

                // TODO use document fragment ?
                for dom in children.iter_mut() {
                    dom.callbacks.leak();

                    element.append_child(&dom.element);

                    if is_inserted {
                        dom.callbacks.trigger_after_insert();
                    }
                }
            },

            VecChange::InsertAt { index, mut value } => {
                // TODO better usize -> u32 conversion
                dom_operations::insert_at(&element, index as u32, &value.element);

                value.callbacks.leak();

                if state.is_inserted.get() {
                    value.callbacks.trigger_after_insert();
                }

                // TODO figure out a way to move this to the top
                state.children.borrow_mut().insert(index, value);
            },

            VecChange::Push { mut value } => {
                element.append_child(&value.element);

                value.callbacks.leak();

                if state.is_inserted.get() {
                    value.callbacks.trigger_after_insert();
                }

                // TODO figure out a way to move this to the top
                state.children.borrow_mut().push(value);
            },

            VecChange::UpdateAt { index, mut value } => {
                // TODO better usize -> u32 conversion
                dom_operations::update_at(&element, index as u32, &value.element);

                value.callbacks.leak();

                if state.is_inserted.get() {
                    value.callbacks.trigger_after_insert();
                }

                // TODO figure out a way to move this to the top
                let mut children = state.children.borrow_mut();

                // TODO test this
                ::std::mem::swap(&mut children[index], &mut value);

                value.callbacks.discard();
            },

            VecChange::RemoveAt { index } => {
                // TODO better usize -> u32 conversion
                dom_operations::remove_at(&element, index as u32);

                state.children.borrow_mut().remove(index).callbacks.discard();
            },

            VecChange::Pop {} => {
                let mut children = state.children.borrow_mut();

                let index = children.len() - 1;

                // TODO create remove_last_child function ?
                // TODO better usize -> u32 conversion
                dom_operations::remove_at(&element, index as u32);

                children.pop().unwrap().callbacks.discard();
            },

            VecChange::Clear {} => {
                dom_operations::remove_all_children(&element);

                let mut children = state.children.borrow_mut();

                for dom in children.drain(..) {
                    dom.callbacks.discard();
                }
            },
        }
    });

    // TODO verify that this will drop `children`
    callbacks.after_remove(handle);
}
