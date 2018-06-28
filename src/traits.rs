use dom_operations;
use dom::DerefFn;
use std::ops::Deref;
use stdweb::Reference;

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


pub trait StyleName {
    fn set_style<A: AsRef<Reference>>(&self, element: &A, value: &str, important: bool);
    fn remove_style<A: AsRef<Reference>>(&self, element: &A);
}

impl<'a> StyleName for &'a str {
    #[inline]
    fn set_style<A: AsRef<Reference>>(&self, element: &A, value: &str, important: bool) {
        if !dom_operations::try_set_style(element, self, value, important) {
            panic!("style is incorrect:\n  name: {}\n  value: {}", self, value);
        }
    }

    #[inline]
    fn remove_style<A: AsRef<Reference>>(&self, element: &A) {
        dom_operations::remove_style(element, self);
    }
}


macro_rules! array_style_name {
    ($size:expr) => {
        impl<'a> StyleName for [&'a str; $size] {
            #[inline]
            fn set_style<A: AsRef<Reference>>(&self, element: &A, value: &str, important: bool) {
                let mut okay = false;

                for name in self.iter() {
                    if dom_operations::try_set_style(element, name, value, important) {
                        okay = true;
                    }
                }

                if !okay {
                    panic!("style is incorrect:\n  names: {}\n  value: {}", self.join(", "), value);
                }
            }

            #[inline]
            fn remove_style<A: AsRef<Reference>>(&self, element: &A) {
                for name in self.iter() {
                    dom_operations::remove_style(element, name);
                }
            }
        }
    };
}

macro_rules! array_style_names {
    ($($size:expr),*) => {
        $(array_style_name!($size);)*
    };
}

array_style_names!(2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32);


// TODO figure out a way to implement this for all of AsRef / Borrow / etc.
// TODO implementations for &String and &mut String
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
