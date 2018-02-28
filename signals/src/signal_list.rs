use futures::Async;
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

    // TODO should this inline ?
    #[inline]
    fn poll(&mut self) -> Async<Option<ListChange<Self::Item>>> {
        self.signal.poll().map(|some| some.map(|change| change.map(|value| (self.callback)(value))))
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
    use super::*;

    struct Tester<A> {
        changes: Vec<ListChange<A>>,
    }

    impl<A> Tester<A> {
        #[inline]
        fn new(changes: Vec<ListChange<A>>) -> Self {
            Self { changes }
        }
    }

    impl<A> SignalList for Tester<A> {
        type Item = A;

        #[inline]
        fn poll(&mut self) -> Async<Option<ListChange<Self::Item>>> {
            if self.changes.len() > 0 {
                Async::Ready(Some(self.changes.remove(0)))

            } else {
                Async::Ready(None)
            }
        }
    }


    #[test]
    fn filter_map_insert_at() {
        let input = Tester::new(vec![
            ListChange::Replace { values: vec![0, 1, 2, 3, 4, 5] },
            ListChange::InsertAt { index: 0, value: 6 },
            ListChange::InsertAt { index: 2, value: 7 },
            ListChange::InsertAt { index: 5, value: 8 },
            ListChange::InsertAt { index: 7, value: 9 },
            ListChange::InsertAt { index: 9, value: 10 },
            ListChange::InsertAt { index: 11, value: 11 },
        ]);

        let mut output = input.filter_map(|x| {
            if x == 3 || x == 4 || x > 5 {
                Some(x + 100)
            } else {
                None
            }
        });

        assert_eq!(output.length, 0);
        assert_eq!(output.indexes, vec![]);

        assert_eq!(output.poll(), Async::Ready(Some(ListChange::Replace {
            values: vec![103, 104],
        })));
        assert_eq!(output.length, 2);
        assert_eq!(output.indexes, vec![None, None, None, Some(0), Some(1), None]);

        assert_eq!(output.poll(), Async::Ready(Some(ListChange::InsertAt { index: 0, value: 106 })));
        assert_eq!(output.length, 3);
        assert_eq!(output.indexes, vec![Some(0), None, None, None, Some(1), Some(2), None]);

        assert_eq!(output.poll(), Async::Ready(Some(ListChange::InsertAt { index: 1, value: 107 })));
        assert_eq!(output.length, 4);
        assert_eq!(output.indexes, vec![Some(0), None, Some(1), None, None, Some(2), Some(3), None]);

        assert_eq!(output.poll(), Async::Ready(Some(ListChange::InsertAt { index: 2, value: 108 })));
        assert_eq!(output.length, 5);
        assert_eq!(output.indexes, vec![Some(0), None, Some(1), None, None, Some(2), Some(3), Some(4), None]);

        assert_eq!(output.poll(), Async::Ready(Some(ListChange::InsertAt { index: 4, value: 109 })));
        assert_eq!(output.length, 6);
        assert_eq!(output.indexes, vec![Some(0), None, Some(1), None, None, Some(2), Some(3), Some(4), Some(5), None]);

        assert_eq!(output.poll(), Async::Ready(Some(ListChange::InsertAt { index: 6, value: 110 })));
        assert_eq!(output.length, 7);
        assert_eq!(output.indexes, vec![Some(0), None, Some(1), None, None, Some(2), Some(3), Some(4), Some(5), Some(6), None]);

        assert_eq!(output.poll(), Async::Ready(Some(ListChange::InsertAt { index: 7, value: 111 })));
        assert_eq!(output.length, 8);
        assert_eq!(output.indexes, vec![Some(0), None, Some(1), None, None, Some(2), Some(3), Some(4), Some(5), Some(6), None, Some(7)]);
    }
}
