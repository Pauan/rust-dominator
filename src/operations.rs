use std::sync::{Arc, Mutex};
use std::future::Future;
use std::iter::IntoIterator;

use discard::{Discard, DiscardOnDrop};
use futures_util::future::ready;
use futures_signals::{cancelable_future, CancelableFutureHandle};
use futures_signals::signal::{Signal, SignalExt};
use futures_signals::signal_vec::{VecDiff, SignalVec, SignalVecExt};
use web_sys::Node;
use wasm_bindgen::UnwrapThrowExt;
use wasm_bindgen_futures::futures_0_3::spawn_local;

use crate::bindings;
use crate::dom::Dom;
use crate::callbacks::Callbacks;


#[inline]
pub(crate) fn spawn_future<F>(future: F) -> DiscardOnDrop<CancelableFutureHandle>
    where F: Future<Output = ()> + 'static {
    // TODO make this more efficient ?
    let (handle, future) = cancelable_future(future, || ());

    spawn_local(future);

    handle
}


#[inline]
pub(crate) fn for_each<A, B>(signal: A, mut callback: B) -> CancelableFutureHandle
    where A: Signal + 'static,
          B: FnMut(A::Item) + 'static {

    DiscardOnDrop::leak(spawn_future(signal.for_each(move |value| {
        callback(value);
        ready(())
    })))
}


#[inline]
fn for_each_vec<A, B>(signal: A, mut callback: B) -> CancelableFutureHandle
    where A: SignalVec + 'static,
          B: FnMut(VecDiff<A::Item>) + 'static {

    DiscardOnDrop::leak(spawn_future(signal.for_each(move |value| {
        callback(value);
        ready(())
    })))
}


#[inline]
pub(crate) fn insert_children_iter<'a, A: IntoIterator<Item = &'a mut Dom>>(element: &Node, callbacks: &mut Callbacks, value: A) {
    for dom in value.into_iter() {
        // TODO can this be made more efficient ?
        callbacks.after_insert.append(&mut dom.callbacks.after_insert);
        callbacks.after_remove.append(&mut dom.callbacks.after_remove);

        bindings::append_child(element, &dom.element);
    }
}


pub(crate) fn insert_children_signal_vec<A>(element: Node, callbacks: &mut Callbacks, signal: A)
    where A: SignalVec<Item = Dom> + 'static {

    // TODO does this create a new struct type every time ?
    struct State {
        is_inserted: bool,
        children: Vec<Dom>,
    }

    // TODO use two separate Arcs ?
    let state = Arc::new(Mutex::new(State {
        is_inserted: false,
        children: vec![],
    }));

    {
        let state = state.clone();

        callbacks.after_insert(move |_| {
            let mut state = state.lock().unwrap_throw();

            if !state.is_inserted {
                state.is_inserted = true;

                for dom in state.children.iter_mut() {
                    dom.callbacks.trigger_after_insert();
                }
            }
        });
    }

    fn process_change(state: &mut State, element: &Node, change: VecDiff<Dom>) {
        match change {
            VecDiff::Replace { values } => {
                // TODO is this correct ?
                if state.children.len() > 0 {
                    bindings::remove_all_children(element);

                    for dom in state.children.drain(..) {
                        dom.callbacks.discard();
                    }
                }

                state.children = values;

                let is_inserted = state.is_inserted;

                for dom in state.children.iter_mut() {
                    dom.callbacks.leak();

                    bindings::append_child(element, &dom.element);

                    if is_inserted {
                        dom.callbacks.trigger_after_insert();
                    }
                }
            },

            VecDiff::InsertAt { index, mut value } => {
                // TODO better usize -> u32 conversion
                bindings::insert_at(element, index as u32, &value.element);

                value.callbacks.leak();

                if state.is_inserted {
                    value.callbacks.trigger_after_insert();
                }

                // TODO figure out a way to move this to the top
                state.children.insert(index, value);
            },

            VecDiff::Push { mut value } => {
                bindings::append_child(element, &value.element);

                value.callbacks.leak();

                if state.is_inserted {
                    value.callbacks.trigger_after_insert();
                }

                // TODO figure out a way to move this to the top
                state.children.push(value);
            },

            VecDiff::UpdateAt { index, mut value } => {
                // TODO better usize -> u32 conversion
                bindings::update_at(element, index as u32, &value.element);

                value.callbacks.leak();

                if state.is_inserted {
                    value.callbacks.trigger_after_insert();
                }

                // TODO figure out a way to move this to the top
                // TODO test this
                ::std::mem::swap(&mut state.children[index], &mut value);

                value.callbacks.discard();
            },

            VecDiff::Move { old_index, new_index } => {
                let value = state.children.remove(old_index);

                bindings::remove_child(element, &value.element);
                // TODO better usize -> u32 conversion
                bindings::insert_at(element, new_index as u32, &value.element);

                state.children.insert(new_index, value);
            },

            VecDiff::RemoveAt { index } => {
                // TODO better usize -> u32 conversion
                bindings::remove_at(element, index as u32);

                state.children.remove(index).callbacks.discard();
            },

            VecDiff::Pop {} => {
                let index = state.children.len() - 1;

                // TODO create remove_last_child function ?
                // TODO better usize -> u32 conversion
                bindings::remove_at(element, index as u32);

                state.children.pop().unwrap_throw().callbacks.discard();
            },

            VecDiff::Clear {} => {
                // TODO is this correct ?
                // TODO is this needed, or is it guaranteed by VecDiff ?
                if state.children.len() > 0 {
                    bindings::remove_all_children(element);

                    for dom in state.children.drain(..) {
                        dom.callbacks.discard();
                    }
                }
            },
        }
    }

    // TODO verify that this will drop `children`
    callbacks.after_remove(for_each_vec(signal, move |change| {
        let mut state = state.lock().unwrap_throw();

        process_change(&mut state, &element, change);
    }));
}
