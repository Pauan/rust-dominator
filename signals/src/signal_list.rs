use futures::Async;
//use std::iter::Iterator;


#[derive(Debug, Clone)]
pub enum ListChange<A> {
    Replace {
        values: Vec<A>,
    },

    InsertAt {
        index: usize,
        value: A,
    },

    RemoveAt {
        index: usize,
    },

    Swap {
        old_index: usize,
        new_index: usize,
    },

    Push {
        value: A,
    },

    Pop {},

    Clear {},
}

impl<A> ListChange<A> {
    // TODO inline this ?
    fn map<B, F>(self, mut callback: F) -> ListChange<B> where F: FnMut(A) -> B {
        match self {
            // TODO figure out a more efficient way of implementing this
            ListChange::Replace { values } => ListChange::Replace { values: values.into_iter().map(callback).collect() },
            ListChange::InsertAt { index, value } => ListChange::InsertAt { index, value: callback(value) },
            ListChange::RemoveAt { index } => ListChange::RemoveAt { index },
            ListChange::Swap { old_index, new_index } => ListChange::Swap { old_index, new_index },
            ListChange::Push { value } => ListChange::Push { value: callback(value) },
            ListChange::Pop {} => ListChange::Pop {},
            ListChange::Clear {} => ListChange::Clear {},
        }
    }
}


pub trait SignalList {
    type Item;

    fn poll(&mut self) -> Async<Option<ListChange<Self::Item>>>;

    #[inline]
    fn map<A, F>(self, callback: F) -> Map<Self, F>
        where F: FnMut(Self::Item) -> A,
              Self: Sized {
        Map {
            signal: self,
            callback,
        }
    }

    /*#[inline]
    fn filter_map<A, F>(self, callback: F) -> Map<Self, F>
        where F: FnMut(Self::Item) -> A,
              Self: Sized {
        Map {
            signal: self,
            callback,
        }
    }*/

    #[inline]
    fn as_mut(&mut self) -> &mut Self {
        self
    }
}


pub struct Map<A, B> {
    signal: A,
    callback: B,
}

impl<A, B, F> SignalList for Map<A, F>
    where A: SignalList,
          F: FnMut(A::Item) -> B {
    type Item = B;

    #[inline]
    fn poll(&mut self) -> Async<Option<ListChange<Self::Item>>> {
        self.signal.poll().map(|some| some.map(|change| change.map(|value| (self.callback)(value))))
    }
}
