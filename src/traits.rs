use std::sync::Arc;
use std::rc::Rc;
use std::borrow::Cow;

pub use animation::AnimatedSignalVec;


pub trait Mixin<A> {
    fn apply(&self, builder: A) -> A;
}

impl<A, F> Mixin<A> for F where F: Fn(A) -> A {
    #[inline]
    fn apply(&self, builder: A) -> A {
        self(builder)
    }
}


// TODO figure out a way to implement this for all of AsRef / Borrow / etc.
pub trait AsStr {
    fn as_str(&self) -> &str;
}

impl AsStr for String {
    #[inline]
    fn as_str(&self) -> &str {
        self.as_str()
    }
}

impl<A: AsStr> AsStr for Box<A> {
    #[inline]
    fn as_str(&self) -> &str {
        (**self).as_str()
    }
}

impl<A: AsStr> AsStr for Arc<A> {
    #[inline]
    fn as_str(&self) -> &str {
        (**self).as_str()
    }
}

impl<A: AsStr> AsStr for Rc<A> {
    #[inline]
    fn as_str(&self) -> &str {
        (**self).as_str()
    }
}

impl<'a, A: AsStr + Clone> AsStr for Cow<'a, A> {
    #[inline]
    fn as_str(&self) -> &str {
        (**self).as_str()
    }
}

impl AsStr for str {
    #[inline]
    fn as_str(&self) -> &str {
        self
    }
}

impl<'a> AsStr for &'a str {
    #[inline]
    fn as_str(&self) -> &str {
        self
    }
}

impl<'a> AsStr for &'a mut str {
    #[inline]
    fn as_str(&self) -> &str {
        self
    }
}


// TODO figure out a way to implement this for all of AsRef / Borrow / etc.
pub trait AsOptionStr {
    fn as_option_str(&self) -> Option<&str>;
}

impl<A: AsStr> AsOptionStr for A {
    #[inline]
    fn as_option_str(&self) -> Option<&str> {
        Some(self.as_str())
    }
}

impl<A: AsStr> AsOptionStr for Option<A> {
    #[inline]
    fn as_option_str(&self) -> Option<&str> {
        self.as_ref().map(|x| x.as_str())
    }
}

/*impl<A: AsStr> AsOptionStr for Box<Option<A>> {
    #[inline]
    fn as_option_str(&self) -> Option<&str> {
        (**self).as_option_str()
    }
}

impl<A: AsStr> AsOptionStr for Arc<Option<A>> {
    #[inline]
    fn as_option_str(&self) -> Option<&str> {
        (**self).as_option_str()
    }
}

impl<A: AsStr> AsOptionStr for Rc<Option<A>> {
    #[inline]
    fn as_option_str(&self) -> Option<&str> {
        (**self).as_option_str()
    }
}

impl<'a, A: AsStr + Clone> AsOptionStr for Cow<'a, Option<A>> {
    #[inline]
    fn as_option_str(&self) -> Option<&str> {
        (**self).as_option_str()
    }
}*/
