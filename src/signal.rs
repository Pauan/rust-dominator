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
    fn map_dedupe<A, B>(self, callback: A) -> MapDedupe<Self, A>
        where A: FnMut(&Self::Value) -> B,
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
    fn and_then<A, B>(self, callback: A) -> Flatten<Map<Self, A>>
        where A: FnMut(Self::Value) -> B,
              B: Signal,
              Self: Sized {
        self.map(callback).flatten()
    }

    // TODO make this more efficient
    fn for_each<A>(self, callback: A) -> DropHandle
        where A: Fn(Self::Value) + 'static,
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
                Async::Ready(value) => {
                    let has_changed = match self.old_value {
                        Some(ref old_value) => *old_value != value,
                        None => true,
                    };

                    if has_changed {
                        let output = (self.callback)(&value);
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
