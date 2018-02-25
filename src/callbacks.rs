use std;


// TODO replace this with FnOnce later
trait IRemoveCallback {
    fn call(self: Box<Self>);
}

impl<F: FnOnce()> IRemoveCallback for F {
    #[inline]
    fn call(self: Box<Self>) {
        self();
    }
}


// TODO replace this with FnOnce later
trait IInsertCallback {
    fn call(self: Box<Self>, &mut Callbacks);
}

impl<F: FnOnce(&mut Callbacks)> IInsertCallback for F {
    #[inline]
    fn call(self: Box<Self>, callbacks: &mut Callbacks) {
        self(callbacks);
    }
}


pub struct InsertCallback(Box<IInsertCallback>);

pub struct RemoveCallback(Box<IRemoveCallback>);

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
pub struct Callbacks {
    pub after_insert: Vec<InsertCallback>,
    pub after_remove: Vec<RemoveCallback>,
    // TODO figure out a better way
    pub(crate) trigger_remove: bool,
}

impl Callbacks {
    #[inline]
    pub fn new() -> Self {
        Self {
            after_insert: vec![],
            after_remove: vec![],
            trigger_remove: true,
        }
    }

    #[inline]
    pub fn after_insert<A: FnOnce(&mut Callbacks) + 'static>(&mut self, callback: A) {
        self.after_insert.push(InsertCallback(Box::new(callback)));
    }

    #[inline]
    pub fn after_remove<A: FnOnce() + 'static>(&mut self, callback: A) {
        self.after_remove.push(RemoveCallback(Box::new(callback)));
    }

    // TODO runtime checks to make sure this isn't called multiple times ?
    #[inline]
    pub fn trigger_after_insert(&mut self) {
        let mut callbacks = Callbacks::new();

        // TODO verify that this is correct
        // TODO is this the most efficient way to accomplish this ?
        std::mem::swap(&mut callbacks.after_remove, &mut self.after_remove);

        for f in self.after_insert.drain(..) {
            f.0.call(&mut callbacks);
        }

        self.after_insert.shrink_to_fit();

        // TODO figure out a better way of verifying this
        assert_eq!(callbacks.after_insert.len(), 0);

        // TODO verify that this is correct
        std::mem::swap(&mut callbacks.after_remove, &mut self.after_remove);
    }

    #[inline]
    fn trigger_after_remove(&mut self) {
        for f in self.after_remove.drain(..) {
            f.0.call();
        }

        // TODO is this a good idea?
        self.after_remove.shrink_to_fit();
    }
}

// TODO use Discard instead
impl Drop for Callbacks {
    #[inline]
    fn drop(&mut self) {
        if self.trigger_remove {
            self.trigger_after_remove();
        }
    }
}
