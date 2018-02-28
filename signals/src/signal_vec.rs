use futures::{Stream, Poll, Async};
//use std::iter::Iterator;


fn increment_indexes(indexes: &mut [Option<usize>]) -> Option<usize> {
    let mut first = None;

    for index in indexes.into_iter() {
        if let Some(i) = *index {
            if let None = first {
                first = Some(i);
            }

            *index = Some(i + 1);
        }
    }

    first
}

fn decrement_indexes(indexes: &mut [Option<usize>]) {
    for index in indexes {
        if let Some(i) = *index {
            *index = Some(i - 1);
        }
    }
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListChange<A> {
    Replace {
        values: Vec<A>,
    },

    InsertAt {
        index: usize,
        value: A,
    },

    UpdateAt {
        index: usize,
        value: A,
    },

    RemoveAt {
        index: usize,
    },

    // TODO
    /*Swap {
        old_index: usize,
        new_index: usize,
    },*/

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
            ListChange::UpdateAt { index, value } => ListChange::UpdateAt { index, value: callback(value) },
            ListChange::RemoveAt { index } => ListChange::RemoveAt { index },
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

    #[inline]
    fn filter_map<A, F>(self, callback: F) -> FilterMap<Self, F>
        where F: FnMut(Self::Item) -> Option<A>,
              Self: Sized {
        FilterMap {
            length: 0,
            indexes: vec![],
            signal: self,
            callback,
        }
    }

    #[inline]
    fn to_stream(self) -> SignalListStream<Self> where Self: Sized {
        SignalListStream {
            signal: self,
        }
    }

    #[inline]
    fn by_ref(&mut self) -> &mut Self {
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

    // TODO should this inline ?
    #[inline]
    fn poll(&mut self) -> Async<Option<ListChange<Self::Item>>> {
        self.signal.poll().map(|some| some.map(|change| change.map(|value| (self.callback)(value))))
    }
}


pub struct SignalListStream<A> {
    signal: A,
}

impl<A: SignalList> Stream for SignalListStream<A> {
    type Item = ListChange<A::Item>;
    type Error = ();

    #[inline]
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.signal.poll() {
            Async::Ready(some) => Ok(Async::Ready(some)),
            Async::NotReady => Ok(Async::NotReady),
        }
    }
}


pub struct FilterMap<A, B> {
    length: usize,
    indexes: Vec<Option<usize>>,
    signal: A,
    callback: B,
}

impl<A, B, F> SignalList for FilterMap<A, F>
    where A: SignalList,
          F: FnMut(A::Item) -> Option<B> {
    type Item = B;

    // TODO figure out a faster implementation of this
    fn poll(&mut self) -> Async<Option<ListChange<Self::Item>>> {
        loop {
            return match self.signal.poll() {
                Async::NotReady => return Async::NotReady,
                Async::Ready(None) => return Async::Ready(None),
                Async::Ready(Some(change)) => match change {
                    ListChange::Replace { values } => {
                        self.length = 0;
                        self.indexes = Vec::with_capacity(values.len());

                        Async::Ready(Some(ListChange::Replace {
                            values: values.into_iter().filter_map(|value| {
                                let value = (self.callback)(value);

                                match value {
                                    Some(_) => {
                                        self.indexes.push(Some(self.length));
                                        self.length += 1;
                                    },
                                    None => {
                                        self.indexes.push(None);
                                    },
                                }

                                value
                            }).collect()
                        }))
                    },

                    ListChange::InsertAt { index, value } => {
                        match (self.callback)(value) {
                            Some(value) => {
                                let new_index = increment_indexes(&mut self.indexes[index..]).unwrap_or(self.length);

                                self.indexes.insert(index, Some(new_index));
                                self.length += 1;

                                Async::Ready(Some(ListChange::InsertAt { index: new_index, value }))
                            },
                            None => {
                                self.indexes.insert(index, None);
                                continue;
                            },
                        }
                    },

                    ListChange::UpdateAt { index, value } => {
                        match (self.callback)(value) {
                            Some(value) => {
                                match self.indexes[index] {
                                    Some(old_index) => {
                                        Async::Ready(Some(ListChange::UpdateAt { index: old_index, value }))
                                    },
                                    None => {
                                        let new_index = increment_indexes(&mut self.indexes[(index + 1)..]).unwrap_or(self.length);

                                        self.indexes[index] = Some(new_index);
                                        self.length += 1;

                                        Async::Ready(Some(ListChange::InsertAt { index: new_index, value }))
                                    },
                                }
                            },
                            None => {
                                match self.indexes[index] {
                                    Some(old_index) => {
                                        self.indexes[index] = None;

                                        decrement_indexes(&mut self.indexes[(index + 1)..]);
                                        self.length -= 1;

                                        Async::Ready(Some(ListChange::RemoveAt { index: old_index }))
                                    },
                                    None => {
                                        continue;
                                    },
                                }
                            },
                        }
                    },

                    ListChange::RemoveAt { index } => {
                        match self.indexes.remove(index) {
                            Some(old_index) => {
                                decrement_indexes(&mut self.indexes[index..]);
                                self.length -= 1;

                                Async::Ready(Some(ListChange::RemoveAt { index: old_index }))
                            },
                            None => {
                                continue;
                            },
                        }
                    },

                    ListChange::Push { value } => {
                        match (self.callback)(value) {
                            Some(value) => {
                                self.indexes.push(Some(self.length));
                                self.length += 1;
                                Async::Ready(Some(ListChange::Push { value }))
                            },
                            None => {
                                self.indexes.push(None);
                                continue;
                            },
                        }
                    },

                    ListChange::Pop {} => {
                        match self.indexes.pop().expect("Cannot pop from empty vec") {
                            Some(_) => {
                                Async::Ready(Some(ListChange::Pop {}))
                            },
                            None => {
                                continue;
                            },
                        }
                    },

                    ListChange::Clear {} => {
                        self.length = 0;
                        self.indexes = vec![];
                        Async::Ready(Some(ListChange::Clear {}))
                    },
                },
            }
        }
    }
}


#[cfg(test)]
mod tests {
    use futures::{Future, Poll, task};
    use super::*;

    struct Tester<A> {
        changes: Vec<Async<ListChange<A>>>,
    }

    impl<A> Tester<A> {
        #[inline]
        fn new(changes: Vec<Async<ListChange<A>>>) -> Self {
            Self { changes }
        }
    }

    impl<A> SignalList for Tester<A> {
        type Item = A;

        #[inline]
        fn poll(&mut self) -> Async<Option<ListChange<Self::Item>>> {
            if self.changes.len() > 0 {
                match self.changes.remove(0) {
                    Async::NotReady => {
                        task::current().notify();
                        Async::NotReady
                    },
                    Async::Ready(change) => Async::Ready(Some(change)),
                }

            } else {
                Async::Ready(None)
            }
        }
    }


    struct TesterFuture<A, B> {
        signal_list: A,
        callback: B,
    }

    impl<A: SignalList, B: FnMut(&mut A, ListChange<A::Item>)> TesterFuture<A, B> {
        #[inline]
        fn new(signal_list: A, callback: B) -> Self {
            Self { signal_list, callback }
        }
    }

    impl<A, B> Future for TesterFuture<A, B>
        where A: SignalList,
              B: FnMut(&mut A, ListChange<A::Item>) {

        type Item = ();
        type Error = ();

        #[inline]
        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            loop {
                return match self.signal_list.poll() {
                    Async::Ready(Some(change)) => {
                        (self.callback)(&mut self.signal_list, change);
                        continue;
                    },
                    Async::Ready(None) => Ok(Async::Ready(())),
                    Async::NotReady => Ok(Async::NotReady),
                }
            }
        }
    }

    fn run<A: SignalList, B: FnMut(&mut A, ListChange<A::Item>) -> C, C>(signal_list: A, mut callback: B) -> Vec<C> {
        let mut changes = vec![];

        TesterFuture::new(signal_list, |signal, change| {
            changes.push(callback(signal, change));
        }).wait().unwrap();

        changes
    }


    #[test]
    fn filter_map() {
        #[derive(Debug, PartialEq, Eq)]
        struct Change {
            length: usize,
            indexes: Vec<Option<usize>>,
            change: ListChange<u32>,
        }

        let input = Tester::new(vec![
            Async::Ready(ListChange::Replace { values: vec![0, 1, 2, 3, 4, 5] }),
            Async::NotReady,
            Async::Ready(ListChange::InsertAt { index: 0, value: 6 }),
            Async::Ready(ListChange::InsertAt { index: 2, value: 7 }),
            Async::NotReady,
            Async::NotReady,
            Async::NotReady,
            Async::Ready(ListChange::InsertAt { index: 5, value: 8 }),
            Async::Ready(ListChange::InsertAt { index: 7, value: 9 }),
            Async::Ready(ListChange::InsertAt { index: 9, value: 10 }),
            Async::NotReady,
            Async::Ready(ListChange::InsertAt { index: 11, value: 11 }),
            Async::NotReady,
            Async::Ready(ListChange::InsertAt { index: 0, value: 0 }),
            Async::NotReady,
            Async::NotReady,
            Async::Ready(ListChange::InsertAt { index: 1, value: 0 }),
            Async::Ready(ListChange::InsertAt { index: 5, value: 0 }),
            Async::NotReady,
            Async::Ready(ListChange::InsertAt { index: 5, value: 12 }),
            Async::NotReady,
            Async::Ready(ListChange::RemoveAt { index: 0 }),
            Async::Ready(ListChange::RemoveAt { index: 0 }),
            Async::NotReady,
            Async::Ready(ListChange::RemoveAt { index: 0 }),
            Async::Ready(ListChange::RemoveAt { index: 1 }),
            Async::NotReady,
            Async::Ready(ListChange::RemoveAt { index: 0 }),
            Async::NotReady,
            Async::Ready(ListChange::RemoveAt { index: 0 }),
        ]);

        let output = input.filter_map(|x| {
            if x == 3 || x == 4 || x > 5 {
                Some(x + 100)
            } else {
                None
            }
        });

        assert_eq!(output.length, 0);
        assert_eq!(output.indexes, vec![]);

        let changes = run(output, |output, change| {
            Change {
                change: change,
                length: output.length,
                indexes: output.indexes.clone(),
            }
        });

        assert_eq!(changes, vec![
            Change { length: 2, indexes: vec![None, None, None, Some(0), Some(1), None], change: ListChange::Replace { values: vec![103, 104] } },
            Change { length: 3, indexes: vec![Some(0), None, None, None, Some(1), Some(2), None], change: ListChange::InsertAt { index: 0, value: 106 } },
            Change { length: 4, indexes: vec![Some(0), None, Some(1), None, None, Some(2), Some(3), None], change: ListChange::InsertAt { index: 1, value: 107 } },
            Change { length: 5, indexes: vec![Some(0), None, Some(1), None, None, Some(2), Some(3), Some(4), None], change: ListChange::InsertAt { index: 2, value: 108 } },
            Change { length: 6, indexes: vec![Some(0), None, Some(1), None, None, Some(2), Some(3), Some(4), Some(5), None], change: ListChange::InsertAt { index: 4, value: 109 } },
            Change { length: 7, indexes: vec![Some(0), None, Some(1), None, None, Some(2), Some(3), Some(4), Some(5), Some(6), None], change: ListChange::InsertAt { index: 6, value: 110 } },
            Change { length: 8, indexes: vec![Some(0), None, Some(1), None, None, Some(2), Some(3), Some(4), Some(5), Some(6), None, Some(7)], change: ListChange::InsertAt { index: 7, value: 111 } },
            Change { length: 9, indexes: vec![None, None, Some(0), None, Some(1), Some(2), None, None, None, Some(3), Some(4), Some(5), Some(6), Some(7), None, Some(8)], change: ListChange::InsertAt { index: 2, value: 112 } },
            Change { length: 8, indexes: vec![None, Some(0), Some(1), None, None, None, Some(2), Some(3), Some(4), Some(5), Some(6), None, Some(7)], change: ListChange::RemoveAt { index: 0 } },
            Change { length: 7, indexes: vec![None, Some(0), None, None, None, Some(1), Some(2), Some(3), Some(4), Some(5), None, Some(6)], change: ListChange::RemoveAt { index: 0 } },
            Change { length: 6, indexes: vec![None, None, None, Some(0), Some(1), Some(2), Some(3), Some(4), None, Some(5)], change: ListChange::RemoveAt { index: 0 } },
        ]);
    }
}
