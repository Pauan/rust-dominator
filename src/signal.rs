use std::rc::Rc;
use std::cell::Cell;
use futures::{Async, Poll};
use futures::future::ok;
use futures::stream::Stream;
use stdweb::PromiseFuture;


pub trait Signal {
    type Value;

    // TODO use Async<Option<Self::Value>> to allow the Signal to end ?
    fn poll(&mut self) -> Async<Self::Value>;

    #[inline]
    fn to_stream(self) -> SignalStream<Self>
        where Self: Sized {
        SignalStream {
            signal: self,
        }
    }

    #[inline]
    fn map<A, B>(self, callback: A) -> Map<Self, A>
        where A: FnMut(Self::Value) -> B,
              Self: Sized {
        Map {
            signal: self,
            callback,
        }
    }

    #[inline]
    fn map2<A, B, C>(self, other: A, callback: B) -> Map2<Self, A, B>
        where A: Signal,
              B: FnMut(&mut Self::Value, &mut A::Value) -> C,
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
        where A: FnMut(&mut Self::Value) -> B,
              Self: Sized {
        MapDedupe {
            old_value: None,
            signal: self,
            callback,
        }
    }

    #[inline]
    fn filter_map<A, B>(self, callback: A) -> FilterMap<Self, A>
        where A: FnMut(Self::Value) -> Option<B>,
              Self: Sized {
        FilterMap {
            signal: self,
            callback,
            first: true,
        }
    }

    #[inline]
    fn flatten(self) -> Flatten<Self>
        where Self::Value: Signal,
              Self: Sized {
        Flatten {
            signal: self,
            inner: None,
        }
    }

    #[inline]
    fn switch<A, B>(self, callback: A) -> Flatten<Map<Self, A>>
        where A: FnMut(Self::Value) -> B,
              B: Signal,
              Self: Sized {
        self.map(callback).flatten()
    }

    // TODO file Rust bug about bad error message when `callback` isn't marked as `mut`
    // TODO make this more efficient
    fn for_each<A>(self, mut callback: A) -> DropHandle
        where A: FnMut(Self::Value) + 'static,
              Self: Sized + 'static {

        let (handle, stream) = drop_handle(self.to_stream());

        PromiseFuture::spawn(
            stream.for_each(move |value| {
                callback(value);
                ok(())
            })
        );

        handle
    }

    #[inline]
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}


pub struct Always<A> {
    value: Option<A>,
}

impl<A> Signal for Always<A> {
    type Value = A;

    #[inline]
    fn poll(&mut self) -> Async<Self::Value> {
        self.value.take().map(Async::Ready).unwrap_or(Async::NotReady)
    }
}

#[inline]
pub fn always<A>(value: A) -> Always<A> {
    Always {
        value: Some(value),
    }
}


// TODO figure out a more efficient way to implement this
#[inline]
fn drop_handle<A: Stream>(stream: A) -> (DropHandle, DropStream<A>) {
    let done: Rc<Cell<bool>> = Rc::new(Cell::new(false));

    let drop_handle = DropHandle {
        done: done.clone(),
    };

    let drop_stream = DropStream {
        done,
        stream,
    };

    (drop_handle, drop_stream)
}


// TODO rename this to something else ?
#[must_use]
pub struct DropHandle {
    done: Rc<Cell<bool>>,
}

// TODO change this to use Drop, but it requires some changes to the after_remove callback system
impl DropHandle {
    #[inline]
    pub fn stop(self) {
        self.done.set(true);
    }
}


struct DropStream<A> {
    done: Rc<Cell<bool>>,
    stream: A,
}

impl<A: Stream> Stream for DropStream<A> {
    type Item = A::Item;
    type Error = A::Error;

    #[inline]
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        if self.done.get() {
            Ok(Async::Ready(None))

        } else {
            self.stream.poll()
        }
    }
}


pub struct SignalStream<A> {
    signal: A,
}

impl<A: Signal> Stream for SignalStream<A> {
    type Item = A::Value;
    // TODO use Void instead ?
    type Error = ();

    #[inline]
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        Ok(self.signal.poll().map(Some))
    }
}


pub struct Map<A, B> {
    signal: A,
    callback: B,
}

impl<A, B, C> Signal for Map<A, B>
    where A: Signal,
          B: FnMut(A::Value) -> C {
    type Value = C;

    #[inline]
    fn poll(&mut self) -> Async<Self::Value> {
        self.signal.poll().map(|value| (self.callback)(value))
    }
}


pub struct Map2<A: Signal, B: Signal, C> {
    signal1: A,
    signal2: B,
    callback: C,
    left: Option<A::Value>,
    right: Option<B::Value>,
}

impl<A, B, C, D> Signal for Map2<A, B, C>
    where A: Signal,
          B: Signal,
          C: FnMut(&mut A::Value, &mut B::Value) -> D {
    type Value = D;

    // TODO inline this ?
    fn poll(&mut self) -> Async<Self::Value> {
        match self.signal1.poll() {
            Async::Ready(mut left) => {
                let output = match self.signal2.poll() {
                    Async::Ready(mut right) => {
                        let output = Async::Ready((self.callback)(&mut left, &mut right));
                        self.right = Some(right);
                        output
                    },

                    Async::NotReady => match self.right {
                        Some(ref mut right) => Async::Ready((self.callback)(&mut left, right)),
                        None => Async::NotReady,
                    },
                };

                self.left = Some(left);

                output
            },

            Async::NotReady => match self.left {
                Some(ref mut left) => match self.signal2.poll() {
                    Async::Ready(mut right) => {
                        let output = Async::Ready((self.callback)(left, &mut right));
                        self.right = Some(right);
                        output
                    },

                    Async::NotReady => Async::NotReady,
                },

                None => Async::NotReady,
            },
        }
    }
}


pub struct MapDedupe<A: Signal, B> {
    old_value: Option<A::Value>,
    signal: A,
    callback: B,
}

impl<A, B, C> Signal for MapDedupe<A, B>
    where A: Signal,
          A::Value: PartialEq,
          // TODO should this use Fn instead ?
          B: FnMut(&A::Value) -> C {

    type Value = C;

    // TODO should this use #[inline] ?
    fn poll(&mut self) -> Async<Self::Value> {
        loop {
            match self.signal.poll() {
                Async::Ready(mut value) => {
                    let has_changed = match self.old_value {
                        Some(ref old_value) => *old_value != value,
                        None => true,
                    };

                    if has_changed {
                        let output = (self.callback)(&mut value);
                        self.old_value = Some(value);
                        return Async::Ready(output);
                    }
                },
                Async::NotReady => return Async::NotReady,
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
          B: FnMut(A::Value) -> Option<C> {
    type Value = Option<C>;

    // TODO should this use #[inline] ?
    #[inline]
    fn poll(&mut self) -> Async<Self::Value> {
        loop {
            match self.signal.poll() {
                Async::Ready(value) => match (self.callback)(value) {
                    Some(value) => {
                        self.first = false;
                        return Async::Ready(Some(value));
                    },
                    None => if self.first {
                        self.first = false;
                        return Async::Ready(None);
                    },
                },
                Async::NotReady => return Async::NotReady,
            }
        }
    }
}


pub struct Flatten<A: Signal> {
    signal: A,
    inner: Option<A::Value>,
}

impl<A> Signal for Flatten<A>
    where A: Signal,
          A::Value: Signal {
    type Value = <<A as Signal>::Value as Signal>::Value;

    #[inline]
    fn poll(&mut self) -> Async<Self::Value> {
        match self.signal.poll() {
            Async::Ready(mut inner) => {
                let poll = inner.poll();
                self.inner = Some(inner);
                poll
            },

            Async::NotReady => match self.inner {
                Some(ref mut inner) => inner.poll(),
                None => Async::NotReady,
            },
        }
    }
}


// TODO verify that this is correct
pub mod unsync {
    use super::Signal;
    use std::rc::{Rc, Weak};
    use std::cell::RefCell;
    use futures::Async;
    use futures::task;


    struct Inner<A> {
        value: Option<A>,
        task: Option<task::Task>,
    }


    pub struct Sender<A> {
        inner: Weak<RefCell<Inner<A>>>,
    }

    impl<A> Sender<A> {
        pub fn set(&self, value: A) -> Result<(), A> {
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


    #[derive(Clone)]
    pub struct Receiver<A> {
        inner: Rc<RefCell<Inner<A>>>,
    }

    impl<A> Signal for Receiver<A> {
        type Value = A;

        #[inline]
        fn poll(&mut self) -> Async<Self::Value> {
            let mut inner = self.inner.borrow_mut();

            // TODO is this correct ?
            match inner.value.take() {
                Some(value) => Async::Ready(value),
                None => {
                    inner.task = Some(task::current());
                    Async::NotReady
                },
            }
        }
    }


    pub fn mutable<A>(initial_value: A) -> (Sender<A>, Receiver<A>) {
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


/*map! {
    let foo = 1,
    let bar = 2,
    let qux = 3 => {
        let corge = 4;
    }
}*/


/*
map!(x, y => x + y)
*/


// TODO should this be hidden from the docs ?
#[doc(hidden)]
#[inline]
pub fn pair_rc<'a, 'b, A, B>(left: &'a mut Rc<A>, right: &'b mut Rc<B>) -> (Rc<A>, Rc<B>) {
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
macro_rules! __internal_map_rc_new {
    ($value:expr) => {
        $crate::signal::Signal::map($value, ::std::rc::Rc::new)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __internal_map1 {
    ($name:ident, $value:expr, $f:expr) => {
        $crate::signal::Signal::map(
            __internal_map_rc_new!($value),
            |ref mut $name| $f
        )
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __internal_map2 {
    ($old_expr:expr, $old_pair:pat, $name:ident, $value:expr, $f:expr) => {
        $crate::signal::Signal::map2(
            $old_expr,
            __internal_map_rc_new!($value),
            |$old_pair, ref mut $name| $f
        )
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __internal_map2_pair {
    ($f:expr, $old_expr:expr, $old_pair:pat, { $($lets:stmt);* }, $name:ident, $t:ty, $value:expr, $($args:tt)+) => {
        __internal_map_args!(
            $f,
            $crate::signal::Signal::map2(
                $old_expr,
                __internal_map_rc_new!($value),
                $crate::signal::pair_rc
            ),
            &mut ($old_pair, ref mut $name),
            { $($lets;)* let $name: $t = __internal_map_clone!($name) },
            $($args)+
        )
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __internal_map_args_start {
    ($f:expr, $name:ident, $value:expr, { $($lets:stmt);* }, $($args:tt)+) => {
        __internal_map_args!(
            $f,
            __internal_map_rc_new!($value),
            ref mut $name,
            { $($lets);* },
            $($args)+
        )
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __internal_map_args {
    ($f:expr, $old_expr:expr, $old_pair:pat, { $($lets:stmt);* }, let $name:ident: $t:ty = $value:expr) => {
        __internal_map2!($old_expr, $old_pair, $name, $value, { $($lets;)* let $name: $t = __internal_map_clone!($name); $f })
    };
    ($f:expr, $old_expr:expr, $old_pair:pat, { $($lets:stmt);* }, let $name:ident = $value:expr) => {
        __internal_map2!($old_expr, $old_pair, $name, $value, { $($lets;)* let $name: Rc<_> = __internal_map_clone!($name); $f })
    };
    ($f:expr, $old_expr:expr, $old_pair:pat, { $($lets:stmt);* }, $name:ident) => {
        __internal_map2!($old_expr, $old_pair, $name, $name, { $($lets;)* let $name: Rc<_> = __internal_map_clone!($name); $f })
    };
    ($f:expr, $old_expr:expr, $old_pair:pat, { $($lets:stmt);* }, let $name:ident: $t:ty = $value:expr, $($args:tt)+) => {
        __internal_map2_pair!($f, $old_expr, $old_pair, { $($lets);* }, $name, $t, $value, $($args)+)
    };
    ($f:expr, $old_expr:expr, $old_pair:pat, { $($lets:stmt);* }, let $name:ident = $value:expr, $($args:tt)+) => {
        __internal_map2_pair!($f, $old_expr, $old_pair, { $($lets);* }, $name, Rc<_>, $value, $($args)+)
    };
    ($f:expr, $old_expr:expr, $old_pair:pat, { $($lets:stmt);* }, $name:ident, $($args:tt)+) => {
        __internal_map2_pair!($f, $old_expr, $old_pair, { $($lets);* }, $name, Rc<_>, $name, $($args)+)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __internal_map {
    ($f:expr, let $name:ident: $t:ty = $value:expr) => {
        __internal_map1!($name, $value, { let $name: $t = __internal_map_clone!($name); $f })
    };
    ($f:expr, let $name:ident = $value:expr) => {
        __internal_map1!($name, $value, { let $name: Rc<_> = __internal_map_clone!($name); $f })
    };
    ($f:expr, $name:ident) => {
        __internal_map1!($name, $name, { let $name: Rc<_> = __internal_map_clone!($name); $f })
    };
    ($f:expr, let $name:ident: $t:ty = $value:expr, $($args:tt)+) => {
        __internal_map_args_start!($f, $name, $value, { let $name: $t = __internal_map_clone!($name) }, $($args)+)
    };
    ($f:expr, let $name:ident = $value:expr, $($args:tt)+) => {
        __internal_map_args_start!($f, $name, $value, { let $name: Rc<_> = __internal_map_clone!($name) }, $($args)+)
    };
    ($f:expr, $name:ident, $($args:tt)+) => {
        __internal_map_args_start!($f, $name, $name, { let $name: Rc<_> = __internal_map_clone!($name) }, $($args)+)
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __internal_map_split {
    (($($before:tt)*), => $f:expr) => {
        __internal_map!($f, $($before)*)
    };
    (($($before:tt)*), $t:tt $($after:tt)*) => {
        __internal_map_split!(($($before)* $t), $($after)*)
    };
}

#[macro_export]
macro_rules! map_rc {
    ($($input:tt)*) => { __internal_map_split!((), $($input)*) };
}


#[cfg(test)]
mod tests {
    use futures::Async;
    use super::{Signal, always};

    #[test]
    fn map_macro_ident_1() {
        let a = always(1);
        let mut s = map_rc!(a => a + 1);
        assert_eq!(s.poll(), Async::Ready(2));
        assert_eq!(s.poll(), Async::NotReady);
    }

    #[test]
    fn map_macro_ident_2() {
        let a = always(1);
        let b = always(2);
        let mut s = map_rc!(a, b => *a + *b);
        assert_eq!(s.poll(), Async::Ready(3));
        assert_eq!(s.poll(), Async::NotReady);
    }

    #[test]
    fn map_macro_ident_3() {
        let a = always(1);
        let b = always(2);
        let c = always(3);
        let mut s = map_rc!(a, b, c => *a + *b + *c);
        assert_eq!(s.poll(), Async::Ready(6));
        assert_eq!(s.poll(), Async::NotReady);
    }

    #[test]
    fn map_macro_ident_4() {
        let a = always(1);
        let b = always(2);
        let c = always(3);
        let d = always(4);
        let mut s = map_rc!(a, b, c, d => *a + *b + *c + *d);
        assert_eq!(s.poll(), Async::Ready(10));
        assert_eq!(s.poll(), Async::NotReady);
    }
}
