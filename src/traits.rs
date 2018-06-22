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


/*pub trait DerefStr {
    type Output: Deref<Target = str>;

    fn deref_str(&self) -> Self::Output;
}

impl DerefStr for String {
    type Output = Self;

    #[inline]
    fn deref_str(&self) -> Self::Output {
        self
    }
}

impl<'a> DerefStr for &'a str {
    type Output = &'a str;

    #[inline]
    fn deref_str(&self) -> Self::Output {
        self
    }
}

impl<A, B> DerefStr for DerefFn<A, B> where B: Fn(&A) -> &str {
    type Output = Self;

    #[inline]
    fn deref_str(&self) -> Self::Output {
        self
    }
}


pub trait DerefOptionStr {
    type Output: Deref<Target = str>;

    fn deref_option_str(&self) -> Option<Self::Output>;
}

impl<A: DerefStr> DerefOptionStr for A {
    type Output = A::Output;

    #[inline]
    fn deref_option_str(&self) -> Option<Self::Output> {
        Some(self.deref_str())
    }
}

impl<A: DerefStr> DerefOptionStr for Option<A> {
    type Output = A::Output;

    #[inline]
    fn deref_option_str(&self) -> Option<Self::Output> {
        self.map(|x| x.deref_str())
    }
}*/



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
