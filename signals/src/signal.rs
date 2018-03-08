use std::rc::{Rc, Weak};
use std::cell::RefCell;
use futures::{Async, Poll, task};
use futures::future::{Future, IntoFuture};
use futures::stream::{Stream, ForEach};
use discard::{Discard, DiscardOnDrop};
use signal_vec::{VecChange, SignalVec};


// TODO add in Done to allow the Signal to end ?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State<A> {
    Changed(A),
    NotChanged,
}

impl<A> State<A> {
    #[inline]
    pub fn map<B, F>(self, f: F) -> State<B> where F: FnOnce(A) -> B {
        match self {
            State::Changed(value) => State::Changed(f(value)),
            State::NotChanged => State::NotChanged,
        }
    }

    #[inline]
    pub fn unwrap(self) -> A {
        match self {
            State::Changed(value) => value,
            State::NotChanged => panic!("State has not changed!"),
        }
    }
}


pub trait Signal {
    type Item;

    fn poll(&mut self) -> State<Self::Item>;

    #[inline]
    fn to_stream(self) -> SignalStream<Self>
        where Self: Sized {
        SignalStream {
            signal: self,
        }
    }

    #[inline]
    fn map<A, B>(self, callback: A) -> Map<Self, A>
        where A: FnMut(Self::Item) -> B,
              Self: Sized {
        Map {
            signal: self,
            callback,
        }
    }

    #[inline]
    fn map2<A, B, C>(self, other: A, callback: B) -> Map2<Self, A, B>
        where A: Signal,
              B: FnMut(&mut Self::Item, &mut A::Item) -> C,
              Self: Sized {
        Map2 {
            signal1: self,
            signal2: other,
            callback,
            left: None,
            right: None,
        }
    }

    #[inline]
    fn map_dedupe<A, B>(self, callback: A) -> MapDedupe<Self, A>
        // TODO should this use & instead of &mut ?
        where A: FnMut(&mut Self::Item) -> B,
              Self: Sized {
        MapDedupe {
            old_value: None,
            signal: self,
            callback,
        }
    }

    #[inline]
    fn filter_map<A, B>(self, callback: A) -> FilterMap<Self, A>
        where A: FnMut(Self::Item) -> Option<B>,
              Self: Sized {
        FilterMap {
            signal: self,
            callback,
            first: true,
        }
    }

    #[inline]
    fn flatten(self) -> Flatten<Self>
        where Self::Item: Signal,
              Self: Sized {
        Flatten {
            signal: self,
            inner: None,
        }
    }

    #[inline]
    fn switch<A, B>(self, callback: A) -> Flatten<Map<Self, A>>
        where A: FnMut(Self::Item) -> B,
              B: Signal,
              Self: Sized {
        self.map(callback).flatten()
    }

    #[inline]
    // TODO file Rust bug about bad error message when `callback` isn't marked as `mut`
    fn for_each<F, U>(self, callback: F) -> ForEach<SignalStream<Self>, F, U>
        where F: FnMut(Self::Item) -> U,
              // TODO allow for errors ?
              U: IntoFuture<Item = (), Error = ()>,
              Self: Sized {

        self.to_stream().for_each(callback)
    }

    #[inline]
    fn to_signal_vec(self) -> SignalSignalVec<Self>
        where Self: Sized {
        SignalSignalVec {
            signal: self
        }
    }

    #[inline]
    fn as_mut(&mut self) -> &mut Self where Self: Sized {
        self
    }
}


impl<F: ?Sized + Signal> Signal for ::std::boxed::Box<F> {
    type Item = F::Item;

    #[inline]
    fn poll(&mut self) -> State<Self::Item> {
        (**self).poll()
    }
}


pub struct Always<A> {
    value: Option<A>,
}

impl<A> Signal for Always<A> {
    type Item = A;

    #[inline]
    fn poll(&mut self) -> State<Self::Item> {
        match self.value.take() {
            Some(value) => State::Changed(value),
            None => State::NotChanged,
        }
    }
}

#[inline]
pub fn always<A>(value: A) -> Always<A> {
    Always {
        value: Some(value),
    }
}


struct CancelableFutureState {
    is_cancelled: bool,
    task: Option<task::Task>,
}


pub struct CancelableFutureHandle {
    state: Weak<RefCell<CancelableFutureState>>,
}

impl Discard for CancelableFutureHandle {
    fn discard(self) {
        if let Some(state) = self.state.upgrade() {
            let mut borrow = state.borrow_mut();

            borrow.is_cancelled = true;

            if let Some(task) = borrow.task.take() {
                drop(borrow);
                task.notify();
            }
        }
    }
}


pub struct CancelableFuture<A, B> {
    state: Rc<RefCell<CancelableFutureState>>,
    future: Option<A>,
    when_cancelled: Option<B>,
}

impl<A, B> Future for CancelableFuture<A, B>
    where A: Future,
          B: FnOnce(A) -> A::Item {

    type Item = A::Item;
    type Error = A::Error;

    // TODO should this inline ?
    #[inline]
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let borrow = self.state.borrow();

        if borrow.is_cancelled {
            let future = self.future.take().unwrap();
            let callback = self.when_cancelled.take().unwrap();
            // TODO figure out how to call the callback immediately when discard is called, e.g. using two Rc<RefCell<>>
            Ok(Async::Ready(callback(future)))

        } else {
            drop(borrow);

            match self.future.as_mut().unwrap().poll() {
                Ok(Async::NotReady) => {
                    self.state.borrow_mut().task = Some(task::current());
                    Ok(Async::NotReady)
                },
                a => a,
            }
        }
    }
}


// TODO figure out a more efficient way to implement this
// TODO this should be implemented in the futures crate
#[inline]
pub fn cancelable_future<A, B>(future: A, when_cancelled: B) -> (DiscardOnDrop<CancelableFutureHandle>, CancelableFuture<A, B>)
    where A: Future,
          B: FnOnce(A) -> A::Item {

    let state = Rc::new(RefCell::new(CancelableFutureState {
        is_cancelled: false,
        task: None,
    }));

    let cancel_handle = DiscardOnDrop::new(CancelableFutureHandle {
        state: Rc::downgrade(&state),
    });

    let cancel_future = CancelableFuture {
        state,
        future: Some(future),
        when_cancelled: Some(when_cancelled),
    };

    (cancel_handle, cancel_future)
}


pub struct SignalStream<A> {
    signal: A,
}

impl<A: Signal> Stream for SignalStream<A> {
    type Item = A::Item;
    // TODO use Void instead ?
    type Error = ();

    #[inline]
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        Ok(match self.signal.poll() {
            State::Changed(value) => Async::Ready(Some(value)),
            State::NotChanged => Async::NotReady,
        })
    }
}


pub struct Map<A, B> {
    signal: A,
    callback: B,
}

impl<A, B, C> Signal for Map<A, B>
    where A: Signal,
          B: FnMut(A::Item) -> C {
    type Item = C;

    #[inline]
    fn poll(&mut self) -> State<Self::Item> {
        self.signal.poll().map(|value| (self.callback)(value))
    }
}


pub struct SignalSignalVec<A> {
    signal: A,
}

impl<A, B> SignalVec for SignalSignalVec<A>
    where A: Signal<Item = Vec<B>> {
    type Item = B;

    #[inline]
    fn poll(&mut self) -> Async<Option<VecChange<B>>> {
        match self.signal.poll() {
            State::Changed(values) => Async::Ready(Some(VecChange::Replace { values })),
            State::NotChanged => Async::NotReady,
        }
    }
}


pub struct Map2<A: Signal, B: Signal, C> {
    signal1: A,
    signal2: B,
    callback: C,
    left: Option<A::Item>,
    right: Option<B::Item>,
}

impl<A, B, C, D> Signal for Map2<A, B, C>
    where A: Signal,
          B: Signal,
          C: FnMut(&mut A::Item, &mut B::Item) -> D {
    type Item = D;

    // TODO inline this ?
    fn poll(&mut self) -> State<Self::Item> {
        match self.signal1.poll() {
            State::Changed(mut left) => {
                let output = match self.signal2.poll() {
                    State::Changed(mut right) => {
                        let output = State::Changed((self.callback)(&mut left, &mut right));
                        self.right = Some(right);
                        output
                    },

                    State::NotChanged => match self.right {
                        Some(ref mut right) => State::Changed((self.callback)(&mut left, right)),
                        None => State::NotChanged,
                    },
                };

                self.left = Some(left);

                output
            },

            State::NotChanged => match self.left {
                Some(ref mut left) => match self.signal2.poll() {
                    State::Changed(mut right) => {
                        let output = State::Changed((self.callback)(left, &mut right));
                        self.right = Some(right);
                        output
                    },

                    State::NotChanged => State::NotChanged,
                },

                None => State::NotChanged,
            },
        }
    }
}


pub struct MapDedupe<A: Signal, B> {
    old_value: Option<A::Item>,
    signal: A,
    callback: B,
}

impl<A, B, C> Signal for MapDedupe<A, B>
    where A: Signal,
          A::Item: PartialEq,
          // TODO should this use & instead of &mut ?
          // TODO should this use Fn instead ?
          B: FnMut(&mut A::Item) -> C {

    type Item = C;

    // TODO should this use #[inline] ?
    fn poll(&mut self) -> State<Self::Item> {
        loop {
            match self.signal.poll() {
                State::Changed(mut value) => {
                    let has_changed = match self.old_value {
                        Some(ref old_value) => *old_value != value,
                        None => true,
                    };

                    if has_changed {
                        let output = (self.callback)(&mut value);
                        self.old_value = Some(value);
                        return State::Changed(output);
                    }
                },
                State::NotChanged => return State::NotChanged,
            }
        }
    }
}


pub struct FilterMap<A, B> {
    signal: A,
    callback: B,
    first: bool,
}

impl<A, B, C> Signal for FilterMap<A, B>
    where A: Signal,
          B: FnMut(A::Item) -> Option<C> {
    type Item = Option<C>;

    // TODO should this use #[inline] ?
    #[inline]
    fn poll(&mut self) -> State<Self::Item> {
        loop {
            return match self.signal.poll() {
                State::Changed(value) => match (self.callback)(value) {
                    Some(value) => {
                        self.first = false;
                        State::Changed(Some(value))
                    },
                    None => if self.first {
                        self.first = false;
                        State::Changed(None)
                    } else {
                        continue;
                    },
                },
                State::NotChanged => State::NotChanged,
            }
        }
    }
}


pub struct Flatten<A: Signal> {
    signal: A,
    inner: Option<A::Item>,
}

impl<A> Signal for Flatten<A>
    where A: Signal,
          A::Item: Signal {
    type Item = <<A as Signal>::Item as Signal>::Item;

    #[inline]
    fn poll(&mut self) -> State<Self::Item> {
        match self.signal.poll() {
            State::Changed(mut inner) => {
                let poll = inner.poll();
                self.inner = Some(inner);
                poll
            },

            State::NotChanged => match self.inner {
                Some(ref mut inner) => inner.poll(),
                None => State::NotChanged,
            },
        }
    }
}


// TODO verify that this is correct
pub mod unsync {
    use super::{Signal, State};
    use std::mem::swap;
    use std::rc::{Rc, Weak};
    use std::cell::{Cell, RefCell, RefMut};
    use futures::task;
    use futures::task::Task;
    use serde::{Serialize, Deserialize, Serializer, Deserializer};


    struct MutableState<A> {
        value: A,
        // TODO use HashMap or BTreeMap instead ?
        receivers: Vec<Weak<MutableSignalState<A>>>,
    }

    struct MutableSignalState<A> {
        has_changed: Cell<bool>,
        task: RefCell<Option<Task>>,
        // TODO change this to Weak later
        state: Rc<RefCell<MutableState<A>>>,
    }


    #[derive(Clone)]
    pub struct Mutable<A>(Rc<RefCell<MutableState<A>>>);

    impl<A> Mutable<A> {
        pub fn new(value: A) -> Self {
            Mutable(Rc::new(RefCell::new(MutableState {
                value,
                receivers: vec![],
            })))
        }

        fn notify(state: &mut RefMut<MutableState<A>>) {
            state.receivers.retain(|receiver| {
                if let Some(receiver) = receiver.upgrade() {
                    receiver.has_changed.set(true);

                    if let Some(task) = receiver.task.borrow_mut().take() {
                        // TODO drop the mutable borrow before calling task.notify()
                        task.notify();
                    }

                    true

                } else {
                    false
                }
            });
        }

        /*pub fn update<F>(&self, f: F) where F: FnOnce(A) -> A {
            let mut state = self.0.borrow_mut();

            state.value = f(state.value);

            Self::notify(&mut state);
        }*/

        pub fn replace(&self, mut value: A) -> A {
            let mut state = self.0.borrow_mut();

            swap(&mut state.value, &mut value);

            Self::notify(&mut state);

            value
        }

        pub fn set(&self, value: A) {
            let mut state = self.0.borrow_mut();

            state.value = value;

            Self::notify(&mut state);
        }
    }

    impl<A: Clone> Mutable<A> {
        #[inline]
        pub fn get(&self) -> A {
            self.0.borrow().value.clone()
        }

        pub fn signal(&self) -> MutableSignal<A> {
            let state = Rc::new(MutableSignalState {
                has_changed: Cell::new(true),
                task: RefCell::new(None),
                state: self.0.clone(),
            });

            self.0.borrow_mut().receivers.push(Rc::downgrade(&state));

            MutableSignal(state)
        }
    }

    impl<T> Serialize for Mutable<T> where T: Serialize {
        #[inline]
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
            self.0.borrow().value.serialize(serializer)
        }
    }

    impl<'de, T> Deserialize<'de> for Mutable<T> where T: Deserialize<'de> {
        #[inline]
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
            T::deserialize(deserializer).map(Mutable::new)
        }
    }

    impl<T: Default> Default for Mutable<T> {
        #[inline]
        fn default() -> Self {
            Mutable::new(Default::default())
        }
    }


    pub struct MutableSignal<A: Clone>(Rc<MutableSignalState<A>>);

    impl<A: Clone> Signal for MutableSignal<A> {
        type Item = A;

        fn poll(&mut self) -> State<Self::Item> {
            if self.0.has_changed.replace(false) {
                State::Changed(self.0.state.borrow().value.clone())

            } else {
                *self.0.task.borrow_mut() = Some(task::current());
                State::NotChanged
            }
        }
    }


    struct Inner<A> {
        value: Option<A>,
        task: Option<task::Task>,
    }

    pub struct Sender<A> {
        inner: Weak<RefCell<Inner<A>>>,
    }

    impl<A> Sender<A> {
        pub fn send(&self, value: A) -> Result<(), A> {
            if let Some(inner) = self.inner.upgrade() {
                let mut inner = inner.borrow_mut();

                inner.value = Some(value);

                if let Some(task) = inner.task.take() {
                    drop(inner);
                    task.notify();
                }

                Ok(())

            } else {
                Err(value)
            }
        }
    }

    pub struct Receiver<A> {
        inner: Rc<RefCell<Inner<A>>>,
    }

    impl<A> Signal for Receiver<A> {
        type Item = A;

        #[inline]
        fn poll(&mut self) -> State<Self::Item> {
            let mut inner = self.inner.borrow_mut();

            // TODO is this correct ?
            match inner.value.take() {
                Some(value) => State::Changed(value),
                None => {
                    inner.task = Some(task::current());
                    State::NotChanged
                },
            }
        }
    }

    pub fn channel<A>(initial_value: A) -> (Sender<A>, Receiver<A>) {
        let inner = Rc::new(RefCell::new(Inner {
            value: Some(initial_value),
            task: None,
        }));

        let sender = Sender {
            inner: Rc::downgrade(&inner),
        };

        let receiver = Receiver {
            inner,
        };

        (sender, receiver)
    }
}


// TODO should this be hidden from the docs ?
#[doc(hidden)]
#[inline]
pub fn pair_clone<'a, 'b, A: Clone, B: Clone>(left: &'a mut A, right: &'b mut B) -> (A, B) {
    (left.clone(), right.clone())
}


#[doc(hidden)]
#[macro_export]
macro_rules! __internal_map_clone {
    ($name:ident) => {
        ::std::clone::Clone::clone($name)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __internal_map2 {
    ($f:expr, $old_pair:pat, $old_expr:expr, { $($lets:stmt);* }, let $name:ident: $t:ty = $value:expr;) => {
        $crate::signal::Signal::map2(
            $old_expr,
            $value,
            |&mut $old_pair, $name| {
                $($lets;)*
                let $name: $t = __internal_map_clone!($name);
                $f
            }
        )
    };
    ($f:expr, $old_pair:pat, $old_expr:expr, { $($lets:stmt);* }, let $name:ident: $t:ty = $value:expr; $($args:tt)+) => {
        __internal_map2!(
            $f,
            ($old_pair, ref mut $name),
            $crate::signal::Signal::map2(
                $old_expr,
                $value,
                $crate::signal::pair_clone
            ),
            { $($lets;)* let $name: $t = __internal_map_clone!($name) },
            $($args)+
        )
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __internal_map {
    ($f:expr, let $name:ident: $t:ty = $value:expr;) => {
        $crate::signal::Signal::map($value, |$name| {
            let $name: $t = $name;
            $f
        })
    };
    ($f:expr, let $name1:ident: $t1:ty = $value1:expr;
              let $name2:ident: $t2:ty = $value2:expr;) => {
        $crate::signal::Signal::map2(
            $value1,
            $value2,
            |$name1, $name2| {
                let $name1: $t1 = __internal_map_clone!($name1);
                let $name2: $t2 = __internal_map_clone!($name2);
                $f
            }
        )
    };
    ($f:expr, let $name:ident: $t:ty = $value:expr; $($args:tt)+) => {
        __internal_map2!(
            $f,
            ref mut $name,
            $value,
            { let $name: $t = __internal_map_clone!($name) },
            $($args)+
        )
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __internal_map_lets {
    ($f:expr, { $($lets:tt)* },) => {
        __internal_map!($f, $($lets)*)
    };
    ($f:expr, { $($lets:tt)* }, let $name:ident: $t:ty = $value:expr, $($args:tt)*) => {
        __internal_map_lets!($f, { $($lets)* let $name: $t = $value; }, $($args)*)
    };
    ($f:expr, { $($lets:tt)* }, let $name:ident = $value:expr, $($args:tt)*) => {
        __internal_map_lets!($f, { $($lets)* let $name: _ = $value; }, $($args)*)
    };
    ($f:expr, { $($lets:tt)* }, $name:ident, $($args:tt)*) => {
        __internal_map_lets!($f, { $($lets)* let $name: _ = $name; }, $($args)*)
    };
}

// TODO this is pretty inefficient, it iterates over the token tree one token at a time
#[doc(hidden)]
#[macro_export]
macro_rules! __internal_map_split {
    (($($before:tt)*), => $f:expr) => {
        __internal_map_lets!($f, {}, $($before)*,)
    };
    (($($before:tt)*), $t:tt $($after:tt)*) => {
        __internal_map_split!(($($before)* $t), $($after)*)
    };
}

#[macro_export]
macro_rules! map_clone {
    ($($input:tt)*) => { __internal_map_split!((), $($input)*) };
}



#[cfg(test)]
mod tests {
    #[test]
    fn map_macro_ident_1() {
        let a = super::always(1);

        let mut s = map_clone!(a => {
            let a: u32 = a;
            a + 1
        });

        assert_eq!(super::Signal::poll(&mut s), super::State::Changed(2));
        assert_eq!(super::Signal::poll(&mut s), super::State::NotChanged);
    }

    #[test]
    fn map_macro_ident_2() {
        let a = super::always(1);
        let b = super::always(2);

        let mut s = map_clone!(a, b => {
            let a: u32 = a;
            let b: u32 = b;
            a + b
        });

        assert_eq!(super::Signal::poll(&mut s), super::State::Changed(3));
        assert_eq!(super::Signal::poll(&mut s), super::State::NotChanged);
    }

    #[test]
    fn map_macro_ident_3() {
        let a = super::always(1);
        let b = super::always(2);
        let c = super::always(3);

        let mut s = map_clone!(a, b, c => {
            let a: u32 = a;
            let b: u32 = b;
            let c: u32 = c;
            a + b + c
        });

        assert_eq!(super::Signal::poll(&mut s), super::State::Changed(6));
        assert_eq!(super::Signal::poll(&mut s), super::State::NotChanged);
    }

    #[test]
    fn map_macro_ident_4() {
        let a = super::always(1);
        let b = super::always(2);
        let c = super::always(3);
        let d = super::always(4);

        let mut s = map_clone!(a, b, c, d => {
            let a: u32 = a;
            let b: u32 = b;
            let c: u32 = c;
            let d: u32 = d;
            a + b + c + d
        });

        assert_eq!(super::Signal::poll(&mut s), super::State::Changed(10));
        assert_eq!(super::Signal::poll(&mut s), super::State::NotChanged);
    }

    #[test]
    fn map_macro_ident_5() {
        let a = super::always(1);
        let b = super::always(2);
        let c = super::always(3);
        let d = super::always(4);
        let e = super::always(5);

        let mut s = map_clone!(a, b, c, d, e => {
            let a: u32 = a;
            let b: u32 = b;
            let c: u32 = c;
            let d: u32 = d;
            let e: u32 = e;
            a + b + c + d + e
        });

        assert_eq!(super::Signal::poll(&mut s), super::State::Changed(15));
        assert_eq!(super::Signal::poll(&mut s), super::State::NotChanged);
    }


    #[test]
    fn map_macro_let_1() {
        let a2 = super::always(1);

        let mut s = map_clone!(let a = a2 => {
            let a: u32 = a;
            a + 1
        });

        assert_eq!(super::Signal::poll(&mut s), super::State::Changed(2));
        assert_eq!(super::Signal::poll(&mut s), super::State::NotChanged);
    }

    #[test]
    fn map_macro_let_2() {
        let a2 = super::always(1);
        let b2 = super::always(2);

        let mut s = map_clone!(let a = a2, let b = b2 => {
            let a: u32 = a;
            let b: u32 = b;
            a + b
        });

        assert_eq!(super::Signal::poll(&mut s), super::State::Changed(3));
        assert_eq!(super::Signal::poll(&mut s), super::State::NotChanged);
    }

    #[test]
    fn map_macro_let_3() {
        let a2 = super::always(1);
        let b2 = super::always(2);
        let c2 = super::always(3);

        let mut s = map_clone!(let a = a2, let b = b2, let c = c2 => {
            let a: u32 = a;
            let b: u32 = b;
            let c: u32 = c;
            a + b + c
        });

        assert_eq!(super::Signal::poll(&mut s), super::State::Changed(6));
        assert_eq!(super::Signal::poll(&mut s), super::State::NotChanged);
    }

    #[test]
    fn map_macro_let_4() {
        let a2 = super::always(1);
        let b2 = super::always(2);
        let c2 = super::always(3);
        let d2 = super::always(4);

        let mut s = map_clone!(let a = a2, let b = b2, let c = c2, let d = d2 => {
            let a: u32 = a;
            let b: u32 = b;
            let c: u32 = c;
            let d: u32 = d;
            a + b + c + d
        });

        assert_eq!(super::Signal::poll(&mut s), super::State::Changed(10));
        assert_eq!(super::Signal::poll(&mut s), super::State::NotChanged);
    }

    #[test]
    fn map_macro_let_5() {
        let a2 = super::always(1);
        let b2 = super::always(2);
        let c2 = super::always(3);
        let d2 = super::always(4);
        let e2 = super::always(5);

        let mut s = map_clone!(let a = a2, let b = b2, let c = c2, let d = d2, let e = e2 => {
            let a: u32 = a;
            let b: u32 = b;
            let c: u32 = c;
            let d: u32 = d;
            let e: u32 = e;
            a + b + c + d + e
        });

        assert_eq!(super::Signal::poll(&mut s), super::State::Changed(15));
        assert_eq!(super::Signal::poll(&mut s), super::State::NotChanged);
    }


    #[test]
    fn map_macro_let_type_1() {
        let a2 = super::always(1);

        let mut s = map_clone! {
            let a: u32 = a2 => {
                let a: u32 = a;
                a + 1
            }
        };

        assert_eq!(super::Signal::poll(&mut s), super::State::Changed(2));
        assert_eq!(super::Signal::poll(&mut s), super::State::NotChanged);
    }

    #[test]
    fn map_macro_let_type_2() {
        let a2 = super::always(1);
        let b2 = super::always(2);

        let mut s = map_clone! {
            let a: u32 = a2,
            let b: u32 = b2 => {
                let a: u32 = a;
                let b: u32 = b;
                a + b
            }
        };

        assert_eq!(super::Signal::poll(&mut s), super::State::Changed(3));
        assert_eq!(super::Signal::poll(&mut s), super::State::NotChanged);
    }

    #[test]
    fn map_macro_let_type_3() {
        let a2 = super::always(1);
        let b2 = super::always(2);
        let c2 = super::always(3);

        let mut s = map_clone! {
            let a: u32 = a2,
            let b: u32 = b2,
            let c: u32 = c2 => {
                let a: u32 = a;
                let b: u32 = b;
                let c: u32 = c;
                a + b + c
            }
        };

        assert_eq!(super::Signal::poll(&mut s), super::State::Changed(6));
        assert_eq!(super::Signal::poll(&mut s), super::State::NotChanged);
    }

    #[test]
    fn map_macro_let_type_4() {
        let a2 = super::always(1);
        let b2 = super::always(2);
        let c2 = super::always(3);
        let d2 = super::always(4);

        let mut s = map_clone! {
            let a: u32 = a2,
            let b: u32 = b2,
            let c: u32 = c2,
            let d: u32 = d2 => {
                let a: u32 = a;
                let b: u32 = b;
                let c: u32 = c;
                let d: u32 = d;
                a + b + c + d
            }
        };

        assert_eq!(super::Signal::poll(&mut s), super::State::Changed(10));
        assert_eq!(super::Signal::poll(&mut s), super::State::NotChanged);
    }

    #[test]
    fn map_macro_let_type_5() {
        let a2 = super::always(1);
        let b2 = super::always(2);
        let c2 = super::always(3);
        let d2 = super::always(4);
        let e2 = super::always(5);

        let mut s = map_clone! {
            let a: u32 = a2,
            let b: u32 = b2,
            let c: u32 = c2,
            let d: u32 = d2,
            let e: u32 = e2 => {
                let a: u32 = a;
                let b: u32 = b;
                let c: u32 = c;
                let d: u32 = d;
                let e: u32 = e;
                a + b + c + d + e
            }
        };

        assert_eq!(super::Signal::poll(&mut s), super::State::Changed(15));
        assert_eq!(super::Signal::poll(&mut s), super::State::NotChanged);
    }
}
