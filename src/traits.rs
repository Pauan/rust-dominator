use dom::DerefFn;
use std::ops::Deref;

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
pub trait IntoStr {
    type Output: Deref<Target = str>;

    fn into_str(self) -> Self::Output;
}

impl IntoStr for String {
    type Output = Self;

    #[inline]
    fn into_str(self) -> Self::Output {
        self
    }
}

impl<'a> IntoStr for &'a str {
    type Output = Self;

    #[inline]
    fn into_str(self) -> Self::Output {
        self
    }
}

impl<'a> IntoStr for &'a mut str {
    type Output = Self;

    #[inline]
    fn into_str(self) -> Self::Output {
        self
    }
}

impl<A, B> IntoStr for DerefFn<A, B> where B: Fn(&A) -> &str {
    type Output = Self;

    #[inline]
    fn into_str(self) -> Self::Output {
        self
    }
}


// TODO figure out a way to implement this for all of AsRef / Borrow / etc.
pub trait IntoOptionStr {
    type Output: Deref<Target = str>;

    fn into_option_str(self) -> Option<Self::Output>;
}

impl<A: IntoStr> IntoOptionStr for A {
    type Output = A::Output;

    #[inline]
    fn into_option_str(self) -> Option<Self::Output> {
        Some(self.into_str())
    }
}

impl<A: IntoStr> IntoOptionStr for Option<A> {
    type Output = A::Output;

    #[inline]
    fn into_option_str(self) -> Option<Self::Output> {
        self.map(|x| x.into_str())
    }
}
