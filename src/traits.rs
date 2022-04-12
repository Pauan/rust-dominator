use std::borrow::Cow;
use crate::dom::RefFn;

pub use crate::animation::AnimatedSignalVec;


pub trait StaticEvent {
    const EVENT_TYPE: &'static str;

    fn unchecked_from_event(event: web_sys::Event) -> Self;
}


#[deprecated(since = "0.3.2", note = "Use the apply or apply_if methods instead")]
pub trait Mixin<A> {
    fn apply(self, builder: A) -> A;
}

#[allow(deprecated)]
impl<A, F> Mixin<A> for F where F: FnOnce(A) -> A {
    #[inline]
    fn apply(self, builder: A) -> A {
        self(builder)
    }
}


// TODO figure out a way to implement this for all of AsRef / Borrow / etc.
// TODO implementations for &String and &mut String
pub trait AsStr {
    #[deprecated(since = "0.5.18", note = "Use with_str instead")]
    fn as_str(&self) -> &str;

    fn with_str<A, F>(&self, f: F) -> A where F: FnOnce(&str) -> A {
        #[allow(deprecated)]
        f(self.as_str())
    }
}

impl<'a, A> AsStr for &'a A where A: AsStr {
    #[inline]
    fn as_str(&self) -> &str {
        #[allow(deprecated)]
        AsStr::as_str(*self)
    }

    #[inline]
    fn with_str<B, F>(&self, f: F) -> B where F: FnOnce(&str) -> B {
        AsStr::with_str(*self, f)
    }
}

impl AsStr for String {
    #[inline]
    fn as_str(&self) -> &str {
        self
    }

    #[inline]
    fn with_str<A, F>(&self, f: F) -> A where F: FnOnce(&str) -> A {
        f(&self)
    }
}

impl AsStr for str {
    #[inline]
    fn as_str(&self) -> &str {
        self
    }

    #[inline]
    fn with_str<A, F>(&self, f: F) -> A where F: FnOnce(&str) -> A {
        f(self)
    }
}

impl<'a> AsStr for &'a str {
    #[inline]
    fn as_str(&self) -> &str {
        self
    }

    #[inline]
    fn with_str<A, F>(&self, f: F) -> A where F: FnOnce(&str) -> A {
        f(self)
    }
}

impl<'a> AsStr for Cow<'a, str> {
    #[inline]
    fn as_str(&self) -> &str {
        &*self
    }

    #[inline]
    fn with_str<A, F>(&self, f: F) -> A where F: FnOnce(&str) -> A {
        f(&*self)
    }
}

impl<A, C> AsStr for RefFn<A, str, C> where C: Fn(&A) -> &str {
    #[inline]
    fn as_str(&self) -> &str {
        self.call_ref()
    }

    #[inline]
    fn with_str<B, F>(&self, f: F) -> B where F: FnOnce(&str) -> B {
        f(self.call_ref())
    }
}


pub trait MultiStr {
    fn find_map<A, F>(&self, f: F) -> Option<A> where F: FnMut(&str) -> Option<A>;

    #[inline]
    fn each<F>(&self, mut f: F) where F: FnMut(&str) {
        let _: Option<()> = self.find_map(|x| {
            f(x);
            None
        });
    }
}

impl<A> MultiStr for A where A: AsStr {
    #[inline]
    fn find_map<B, F>(&self, f: F) -> Option<B> where F: FnMut(&str) -> Option<B> {
        self.with_str(f)
    }
}

// TODO it would be great to use IntoIterator instead, and then we can replace the array implementations with it
/*impl<'a, A> MultiStr for &'a [A] where A: AsStr {
    #[inline]
    fn any<F>(&self, mut f: F) -> bool where F: FnMut(&str) -> bool {
        self.iter().any(|x| f(x.as_str()))
    }
}*/

// TODO it would be great to use IntoIterator or Iterator instead
impl<'a, A, C> MultiStr for RefFn<A, [&'a str], C> where C: Fn(&A) -> &[&'a str] {
    #[inline]
    fn find_map<B, F>(&self, mut f: F) -> Option<B> where F: FnMut(&str) -> Option<B> {
        self.call_ref().iter().find_map(|x| f(x))
    }
}

macro_rules! array_multi_str {
    ($size:expr) => {
        impl<A> MultiStr for [A; $size] where A: AsStr {
            #[inline]
            fn find_map<B, F>(&self, mut f: F) -> Option<B> where F: FnMut(&str) -> Option<B> {
                self.iter().find_map(|x| x.with_str(|x| f(x)))
            }
        }
    };
}

macro_rules! array_multi_strs {
    ($($size:expr),*) => {
        $(array_multi_str!($size);)*
    };
}

array_multi_strs!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32);


pub trait OptionStr {
    type Output;

    fn into_option(self) -> Option<Self::Output>;
}

impl<A> OptionStr for A where A: MultiStr {
    type Output = A;

    #[inline]
    fn into_option(self) -> Option<A> {
        Some(self)
    }
}

impl<A> OptionStr for Option<A> where A: MultiStr {
    type Output = A;

    #[inline]
    fn into_option(self) -> Option<A> {
        self
    }
}
