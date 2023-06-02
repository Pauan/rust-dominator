use std::rc::Rc;
use std::sync::Arc;
use std::borrow::BorrowMut;
use futures_signals::signal::{Signal};
use futures_signals::signal_vec::SignalVec;
use web_sys::Node;

use crate::dom::{Dom, DomBuilder};
use crate::traits::*;

#[cfg(doc)]
use crate::{fragment, box_fragment};


/// A fragment is a collection of children which can be inserted into a [`DomBuilder`].
///
/// See the documentation for [`fragment!`] for more details.
pub trait Fragment {
    fn apply<'a>(&self, dom: FragmentBuilder<'a>) -> FragmentBuilder<'a>;
}

impl<A> Fragment for &A where A: Fragment + ?Sized {
    #[inline]
    fn apply<'a>(&self, dom: FragmentBuilder<'a>) -> FragmentBuilder<'a> {
        (*self).apply(dom)
    }
}

impl<A> Fragment for Box<A> where A: Fragment + ?Sized {
    #[inline]
    fn apply<'a>(&self, dom: FragmentBuilder<'a>) -> FragmentBuilder<'a> {
        (**self).apply(dom)
    }
}

impl<A> Fragment for Rc<A> where A: Fragment + ?Sized {
    #[inline]
    fn apply<'a>(&self, dom: FragmentBuilder<'a>) -> FragmentBuilder<'a> {
        (**self).apply(dom)
    }
}

impl<A> Fragment for Arc<A> where A: Fragment + ?Sized {
    #[inline]
    fn apply<'a>(&self, dom: FragmentBuilder<'a>) -> FragmentBuilder<'a> {
        (**self).apply(dom)
    }
}


/// A boxed [`Fragment`]. See the documentation for [`box_fragment!`] for more details.
pub type BoxFragment = Box<dyn Fragment + Send + Sync>;


// TODO better warning message for must_use
/// This is used by the [`fragment!`] and [`box_fragment!`] macros.
#[must_use]
#[derive(Debug)]
pub struct FragmentBuilder<'a>(pub(crate) DomBuilder<&'a Node>);

impl<'a> FragmentBuilder<'a> {
    // TODO experiment with giving the closure &Self instead, to make it impossible to return a different element
    #[inline]
    pub fn apply<F>(self, f: F) -> Self where F: FnOnce(Self) -> Self {
        f(self)
    }

    #[inline]
    pub fn apply_if<F>(self, test: bool, f: F) -> Self where F: FnOnce(Self) -> Self {
        if test {
            f(self)

        } else {
            self
        }
    }

    /// Inserts the [`Fragment`] into this [`FragmentBuilder`]. This is the same as [`DomBuilder::fragment`].
    #[inline]
    #[track_caller]
    pub fn fragment<F>(self, fragment: &F) -> Self where F: Fragment {
        fragment.apply(self)
    }

    #[inline]
    #[track_caller]
    pub fn text(self, value: &str) -> Self {
        Self(self.0.text(value))
    }

    #[inline]
    #[track_caller]
    pub fn text_signal<B, C>(self, value: C) -> Self
        where B: AsStr,
              C: Signal<Item = B> + 'static {
        Self(self.0.text_signal(value))
    }

    #[inline]
    #[track_caller]
    pub fn child<B: BorrowMut<Dom>>(self, child: B) -> Self {
        Self(self.0.child(child))
    }

    #[inline]
    #[track_caller]
    pub fn child_signal<B>(self, child: B) -> Self
        where B: Signal<Item = Option<Dom>> + 'static {
        Self(self.0.child_signal(child))
    }

    // TODO figure out how to make this owned rather than &mut
    #[inline]
    #[track_caller]
    pub fn children<B: BorrowMut<Dom>, C: IntoIterator<Item = B>>(self, children: C) -> Self {
        Self(self.0.children(children))
    }

    #[inline]
    #[track_caller]
    pub fn children_signal_vec<B>(self, children: B) -> Self
        where B: SignalVec<Item = Dom> + 'static {
        Self(self.0.children_signal_vec(children))
    }
}


/// Creates a [`Fragment`] which can be inserted into a [`DomBuilder`].
///
/// A fragment is a collection of children, you can use all of the methods in [`FragmentBuilder`]:
///
/// ```rust
/// let x = fragment!({
///     .text("foo")
///     .child(html!("div", { ... }))
///     .children_signal_vec(...)
/// });
/// ```
///
/// You can then insert the fragment into a [`DomBuilder`]:
///
/// ```rust
/// html!("div", {
///     .fragment(&x)
/// })
/// ```
///
/// The fragment is inlined, so it is exactly the same as if you had written this,
/// there is no performance cost:
///
/// ```rust
/// html!("div", {
///     .text("foo")
///     .child(html!("div", ...))
///     .children_signal_vec(...)
/// })
/// ```
///
/// The same fragment can be inserted multiple times, and it can be
/// inserted into multiple different [`DomBuilder`].
///
/// Fragments are very useful for passing children into another component:
///
/// ```rust
/// Foo::render(&state.foo, fragment!({
///     .text("Hello!")
/// }))
/// ```
///
/// If you need to store the fragment inside of a `struct` or `static` then
/// you must use [`box_fragment!`] instead.
///
/// # Syntax
///
/// There are three syntaxes for fragment:
///
/// 1. `fragment!()` creates an empty fragment.
///
/// 2. `fragment!({ ... })` creates a normal fragment.
///
/// 3. `fragment!(move { ... })` creates a fragment which `move`s the
///    outer variables into the fragment, just like a closure.
///
/// When returning a fragment from a function, you will usually need to use the `move` syntax:
///
/// ```rust
/// fn my_fragment() -> impl Fragment {
///     let x = some_string();
///
///     fragment!(move {
///         .text(&x)
///     })
/// }
/// ```
#[macro_export]
macro_rules! fragment {
    () => {
        $crate::__internal::fragment(|dom| dom)
    };
    (move { $($input:tt)* }) => {
        $crate::__internal::fragment(move |dom| $crate::apply_methods!(dom, { $($input)* }))
    };
    ({ $($input:tt)* }) => {
        $crate::__internal::fragment(|dom| $crate::apply_methods!(dom, { $($input)* }))
    };
}


/// The same as [`fragment!`] except it returns a [`BoxFragment`].
///
/// A [`BoxFragment`] can be stored in a `struct` or `static`:
///
/// ```rust
/// static FOO: Lazy<BoxFragment> = Lazy::new(|| box_fragment!({ ... }));
/// ```
///
/// [`fragment!`] is zero-cost, but `box_fragment!` is different: it has a performance cost,
/// because it must heap-allocate the fragment and do dynamic dispatch.
#[macro_export]
macro_rules! box_fragment {
    () => {
        $crate::__internal::box_fragment(|dom| dom)
    };
    (move { $($input:tt)* }) => {
        $crate::__internal::box_fragment(move |dom| $crate::apply_methods!(dom, { $($input)* }))
    };
    ({ $($input:tt)* }) => {
        $crate::__internal::box_fragment(|dom| $crate::apply_methods!(dom, { $($input)* }))
    };
}
