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


pub trait AsStr {
    fn as_str(&self) -> &str;
}

impl AsStr for String {
    #[inline]
    fn as_str(&self) -> &str {
        self.as_str()
    }
}

impl<'a> AsStr for &'a str {
    #[inline]
    fn as_str(&self) -> &str {
        self
    }
}


pub trait AsOptionStr {
    fn as_option_str(&self) -> Option<&str>;
}

impl AsOptionStr for Option<String> {
    #[inline]
    fn as_option_str(&self) -> Option<&str> {
        self.as_ref().map(|x| x.as_str())
    }
}

impl<'a> AsOptionStr for Option<&'a str> {
    #[inline]
    fn as_option_str(&self) -> Option<&str> {
        *self
    }
}
