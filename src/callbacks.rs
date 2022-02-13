use discard::Discard;
use std;

// TODO replace this with FnOnce later
trait IInsertCallback {
    fn call(self: Box<Self>, callbacks: &mut Callbacks);
}

impl<F: FnOnce(&mut Callbacks)> IInsertCallback for F {
    #[inline]
    fn call(self: Box<Self>, callbacks: &mut Callbacks) {
        self(callbacks);
    }
}

// TODO a bit gross
trait IRemove {
    fn remove(self: Box<Self>);
}

impl<A: Discard> IRemove for A {
    #[inline]
    fn remove(self: Box<Self>) {
        self.discard();
    }
}

pub(crate) struct InsertCallback(Box<dyn IInsertCallback>);

// TODO is there a more efficient way of doing this ?
pub(crate) struct RemoveCallback(Box<dyn IRemove>);

impl std::fmt::Debug for InsertCallback {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "InsertCallback")
    }
}

impl std::fmt::Debug for RemoveCallback {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "RemoveCallback")
    }
}

#[derive(Debug)]
pub(crate) struct Callbacks {
    pub(crate) after_insert: Vec<InsertCallback>,
    pub(crate) after_remove: Vec<RemoveCallback>,
    trigger_remove: bool,
}

impl Callbacks {
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            after_insert: vec![],
            after_remove: vec![],
            trigger_remove: true,
        }
    }

    #[inline]
    pub(crate) fn after_insert<A: FnOnce(&mut Callbacks) + 'static>(&mut self, callback: A) {
        self.after_insert.push(InsertCallback(Box::new(callback)));
    }

    #[inline]
    pub(crate) fn after_remove<A: Discard + 'static>(&mut self, value: A) {
        self.after_remove.push(RemoveCallback(Box::new(value)));
    }

    // TODO runtime checks to make sure this isn't called multiple times ?
    #[inline]
    pub(crate) fn trigger_after_insert(&mut self) {
        if !self.after_insert.is_empty() {
            let mut callbacks = Callbacks::new();

            // TODO verify that this is correct
            // TODO is this the most efficient way to accomplish this ?
            std::mem::swap(&mut callbacks.after_remove, &mut self.after_remove);

            for f in self.after_insert.drain(..) {
                f.0.call(&mut callbacks);
            }

            // TODO verify that this is correct
            self.after_insert = vec![];

            // TODO figure out a better way of verifying this
            assert_eq!(callbacks.after_insert.len(), 0);

            // TODO verify that this is correct
            // TODO what if `callbacks` is leaked ?
            std::mem::swap(&mut callbacks.after_remove, &mut self.after_remove);
        }
    }

    #[inline]
    fn trigger_after_remove(&mut self) {
        for f in self.after_remove.drain(..) {
            f.0.remove();
        }
    }

    #[inline]
    pub(crate) fn leak(&mut self) {
        self.trigger_remove = false;
    }
}

// TODO use DiscardOnDrop instead
impl Drop for Callbacks {
    #[inline]
    fn drop(&mut self) {
        if self.trigger_remove {
            self.trigger_after_remove();
        }
    }
}

impl Discard for Callbacks {
    #[inline]
    fn discard(mut self) {
        self.trigger_after_remove();
    }
}
