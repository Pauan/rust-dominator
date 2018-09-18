use dom::RefFn;

pub use animation::AnimatedSignalVec;


pub trait Mixin<A> {
    fn apply(self, builder: A) -> A;
}

impl<A, F> Mixin<A> for F where F: FnOnce(A) -> A {
    #[inline]
    fn apply(self, builder: A) -> A {
        self(builder)
    }
}


// TODO figure out a way to implement this for all of AsRef / Borrow / etc.
// TODO implementations for &String and &mut String
pub trait AsStr {
    fn as_str(&self) -> &str;
}

impl<'a, A> AsStr for &'a A where A: AsStr {
    #[inline]
    fn as_str(&self) -> &str {
        AsStr::as_str(*self)
    }
}

impl<'a, A> AsStr for &'a mut A where A: AsStr {
    #[inline]
    fn as_str(&self) -> &str {
        AsStr::as_str(*self)
    }
}

impl<A> AsStr for Box<A> where A: AsStr {
    #[inline]
    fn as_str(&self) -> &str {
        AsStr::as_str(&**self)
    }
}

impl AsStr for String {
    #[inline]
    fn as_str(&self) -> &str {
        self
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

impl<A, C> AsStr for RefFn<A, str, C> where C: Fn(&A) -> &str {
    #[inline]
    fn as_str(&self) -> &str {
        self.call_ref()
    }
}


pub trait MultiStr {
    fn any<F>(&self, f: F) -> bool where F: FnMut(&str) -> bool;

    #[inline]
    fn each<F>(&self, mut f: F) where F: FnMut(&str) {
        self.any(|x| {
            f(x);
            false
        });
    }
}

impl<A> MultiStr for A where A: AsStr {
    #[inline]
    fn any<F>(&self, mut f: F) -> bool where F: FnMut(&str) -> bool {
        f(self.as_str())
    }
}

// TODO it would be great to use IntoIterator instead, and then we can replace the array implementations with it
impl<'a, A> MultiStr for &'a [A] where A: AsStr {
    #[inline]
    fn any<F>(&self, mut f: F) -> bool where F: FnMut(&str) -> bool {
        self.iter().any(|x| f(x.as_str()))
    }
}

// TODO it would be great to use IntoIterator or Iterator instead
impl<'a, A, C> MultiStr for RefFn<A, [&'a str], C> where C: Fn(&A) -> &[&'a str] {
    #[inline]
    fn any<F>(&self, mut f: F) -> bool where F: FnMut(&str) -> bool {
        self.call_ref().iter().any(|x| f(x))
    }
}

macro_rules! array_multi_str {
    ($size:expr) => {
        impl<A> MultiStr for [A; $size] where A: AsStr {
            #[inline]
            fn any<F>(&self, mut f: F) -> bool where F: FnMut(&str) -> bool {
                self.iter().any(|x| f(x.as_str()))
            }
        }
    };
}

macro_rules! array_multi_strs {
    ($($size:expr),*) => {
        $(array_multi_str!($size);)*
    };
}

array_multi_strs!(2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32);


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
