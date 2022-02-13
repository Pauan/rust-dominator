/// Add by-value methods or macros to a value.
///
/// In addition, allows you to call macros like you would call a function. This macro is used
/// throughout `dominator`.
///
/// # Examples
///
/// ```
/// # use dominator::apply_methods;
/// let x = apply_methods!(10i8, {
///     .checked_add(10)
///     .unwrap()
///     .checked_add(20)
/// });
///
/// assert_eq!(x, Some(40))
/// ```
#[macro_export]
macro_rules! apply_methods {
    ($this:expr, {}) => {
        $this
    };
    ($this:expr, { .$name:ident!($($args:tt)*) $($rest:tt)* }) => {{
        let this = $this;
        let this = $name!(this, $($args)*);
        $crate::apply_methods!(this, { $($rest)* })
    }};
    ($this:expr, { .$name:ident($($args:expr),*) $($rest:tt)* }) => {{
        let this = $this.$name($($args),*);
        $crate::apply_methods!(this, { $($rest)* })
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __internal_builder {
    ($default:ty, $make:ident, $kind:expr) => {
        $crate::__internal_builder!($default, $make, $kind => $default, {})
    };
    ($default:ty, $make:ident, $kind:expr, $($rest:tt)*) => {
        $crate::__internal_builder!($default, $make, $kind => $default, $($rest)*)
    };
    ($default:ty, $make:ident, $kind:expr => $t:ty) => {
        $crate::__internal_builder!($default, $make, $kind => $t, {})
    };
    ($default:ty, $make:ident, $kind:expr => $t:ty, $($methods:tt)*) => {{
        let builder = $crate::DomBuilder::<$t>::$make($kind);
        let output = $crate::apply_methods!(builder, $($methods)*);
        $crate::DomBuilder::into_dom(output)
    }};
}

/// Lifts a sequence of funtions over the node we are currently constructing.
///
/// # Examples
///
/// ```no_run
/// use dominator::{html, with_node, events};
/// use web_sys::HtmlInputElement;
///
/// let dom = html!("input" => HtmlInputElement, {
///     .with_node!(el => {
///         .event(move |_: events::Change| {
///             // respond to input change here
///         })
///     })
/// });
/// ```
#[macro_export]
macro_rules! with_node {
    ($this:ident, $name:ident => { $($methods:tt)* }) => {{
        let $name = $crate::DomBuilder::__internal_element(&$this);
        $crate::apply_methods!($this, { $($methods)* })
    }};
}

/// Like `apply_methods!`, but only runs methods if `#[cfg($cfg)]`.
///
/// # Examples
///
/// ```
/// # use dominator::with_cfg;
/// let raw = 10i8;
/// let x = with_cfg!(raw, unix, {
///     .checked_add(10)
///     .unwrap()
/// });
///
/// #[cfg(unix)]
/// assert_eq!(x, 20);
/// #[cfg(not(unix))]
/// assert_eq!(x, 10);
/// ```
#[macro_export]
macro_rules! with_cfg {
    ($this:ident, $cfg:meta, { $($methods:tt)* }) => {{
        #[cfg($cfg)]
        let this = $crate::apply_methods!($this, { $($methods)* });

        #[cfg(not($cfg))]
        let this = $this;

        this
    }};
}

/// Allows you to use `dominator` to create web components.
#[macro_export]
macro_rules! shadow_root {
    ($this:ident, $mode:expr => { $($methods:tt)* }) => {{
        let shadow = $crate::DomBuilder::__internal_shadow_root(&$this, $mode);
        let shadow = $crate::apply_methods!(shadow, { $($methods)* });
        $crate::DomBuilder::__internal_transfer_callbacks($this, shadow)
    }};
}

/// Create a new [`Dom`][crate::Dom] from a description of the html you want.
///
/// This is the main way to construct reactive views of your data.
///
/// # Examples
///
/// ```no_run
/// use dominator::html;
///
/// let dom = html!("div", {
///     .children(&mut [
///         html!("h1", {
///             .text("Title")
///         }),
///         html!("p", {
///             .text("para 1")
///         }),
///         html!("p", {
///             .text("para 2")
///         })
///     ])
/// });
/// ```
#[macro_export]
macro_rules! html {
    ($($args:tt)+) => {
        $crate::__internal_builder!($crate::__internal::HtmlElement, new_html, $($args)+)
    };
}

/// Create a new [`Dom`][crate::Dom] from a description of your svg.
///
/// Works similarly to [`html`][crate::html].
#[macro_export]
macro_rules! svg {
    ($($args:tt)+) => {
        $crate::__internal_builder!($crate::__internal::SvgElement, new_svg, $($args)+)
    };
}

/// Applies the given methods to the given node.
///
/// Which methods are allowed will depend on the type of the node, but it will generally be a
/// subtype of `web_sys::Node`.
///
/// [`html`][crate::html] and [`svg`][crate::svg] use the [`DomBuilder`][crate::DomBuilder]
/// internally.
#[macro_export]
macro_rules! dom_builder {
    ($node:expr, { $($methods:tt)* }) => {{
        let builder = $crate::DomBuilder::new($node);
        let output = $crate::apply_methods!(builder, { $($methods)* });
        $crate::DomBuilder::into_dom(output)
    }};
}

/// Adds an entry with the given selector and rules to a stylesheet in the document `<head>`.
///
/// # Examples
///
/// ```no_run
/// # use dominator::{stylesheet, class};
/// stylesheet!("li .test", {
///     .style("color", "black")
///     .style("padding", "10px 0")
/// })
/// ```
///
/// will result in
///
/// ```css
/// li .test {
///     color: black;
///     padding: 10px 0;
/// }
/// ```
///
/// being in a head style element.
#[macro_export]
macro_rules! stylesheet {
    ($rule:expr) => {
        $crate::stylesheet!($rule, {})
    };
    ($rule:expr, { $($methods:tt)* }) => {
        $crate::StylesheetBuilder::__internal_done($crate::apply_methods!($crate::StylesheetBuilder::__internal_new($rule), { $($methods)* }))
    };
}

/// Create a class with the given styles and appends it to a `<style>` in the document `<head>`.
///
/// The name of the class is randomly generated and returned from the macro, meaning it can then be
/// used in html construction. The class is never removed from the document head, so be careful to
/// re-use existing classes rather than making new ones where possible.
///
/// # Examples
///
/// ```no_run
/// // It's often convenient to store the class name in a static variable, which ensures the class
/// // is only created once.
/// use once_cell::sync::Lazy;
/// # use dominator::{html, class};
/// static CLASS: Lazy<String> = Lazy::new(|| class! {
///     .style("display", "inline-block")
///     .style("padding", "10px")
/// });
///
/// html!("div", {
///     .class(&*CLASS)
/// });
/// ```
#[macro_export]
macro_rules! class {
    ($($methods:tt)*) => {{
        $crate::ClassBuilder::__internal_done($crate::apply_methods!($crate::ClassBuilder::__internal_new(), { $($methods)* }))
    }};
}

/// Used within [`class!`][crate::class] to add rules to a pseudo-selector.
///
/// # Examples
///
/// ```no_run
/// use once_cell::sync::Lazy;
/// # use dominator::{html, class, pseudo};
/// static CLASS: Lazy<String> = Lazy::new(|| class! {
///     .style("display", "inline-block")
///     .style("padding", "10px")
///     .pseudo!(":hover", {
///         .style("background-color", "yellow")
///     })
/// });
///
/// html!("div", {
///     .class(&*CLASS)
/// });
#[macro_export]
macro_rules! pseudo {
    ($this:ident, $rules:expr) => {
        $crate::pseudo!($this, $rules, {})
    };
    ($this:ident, $rules:expr, { $($methods:tt)* }) => {{
        $crate::stylesheet!($crate::__internal::Pseudo::new($crate::ClassBuilder::__internal_class_name(&$this), $rules), { $($methods)* });
        $this
    }};
}

// TODO this is pretty inefficient, it iterates over the token tree one token at a time
// TODO this should only work for ::std::clone::Clone::clone
#[doc(hidden)]
#[macro_export]
macro_rules! __internal_clone_split {
    (($($x:ident)*), $t:ident => $y:expr) => {{
        $(let $x = $x.clone();)*
        let $t = $t.clone();
        $y
    }};
    (($($x:ident)*), $t:ident, $($after:tt)*) => {
        $crate::__internal_clone_split!(($($x)* $t), $($after)*)
    };
}

/// A helper to clone values that are then moved into a closure
///
/// # Examples
///
/// ```
/// use std::{thread, sync::{Arc, atomic::{AtomicUsize, Ordering}}};
/// # use dominator::clone;
/// let x = Arc::new(AtomicUsize::new(2));
/// let handle = thread::spawn(clone!(x => move || {
///     x.fetch_add(1, Ordering::Relaxed);
/// }));
/// handle.join().unwrap();
/// assert_eq!(x.load(Ordering::Relaxed), 3);
/// ```
// TODO move into gloo ?
#[macro_export]
macro_rules! clone {
    ($($input:tt)*) => { $crate::__internal_clone_split!((), $($input)*) };
}
