#[cfg(doc)]
use crate::{DomBuilder, Dom, StylesheetBuilder, ClassBuilder};


#[doc(hidden)]
#[macro_export]
macro_rules! __internal_apply_methods_loop {
    ((), $this:expr, {}) => {
        $this
    };

    ((), $this:expr, { . $name:ident ( $($args:expr),* ) $($rest:tt)* }) => {{
        let this = $this.$name($($args),*);
        $crate::__internal_apply_methods_loop!((), this, { $($rest)* })
    }};
    ((), $this:expr, { . $name:ident :: < $($types:ty),* >( $($args:expr),* ) $($rest:tt)* }) => {{
        let this = $this.$name::<$($types),*>($($args),*);
        $crate::__internal_apply_methods_loop!((), this, { $($rest)* })
    }};

    ((), $this:expr, { . $name:ident $($rest:tt)* }) => {
        $crate::__internal_apply_methods_loop!(($name), $this, { $($rest)* })
    };
    ((), $this:expr, { . :: $name:ident $($rest:tt)* }) => {
        $crate::__internal_apply_methods_loop!((:: $name), $this, { $($rest)* })
    };

    (($($path:tt)+), $this:expr, { :: $name:ident $($rest:tt)* }) => {
        $crate::__internal_apply_methods_loop!(($($path)+ :: $name), $this, { $($rest)* })
    };
    (($($path:tt)+), $this:expr, { ! ( $($args:tt)* ) $($rest:tt)* }) => {{
        let this = $this;
        let this = $($path)+!(this, $($args)*);
        $crate::__internal_apply_methods_loop!((), this, { $($rest)* })
    }};
    (($($path:tt)+), $this:expr, { ! [ $($args:tt)* ] $($rest:tt)* }) => {{
        let this = $this;
        let this = $($path)+![this, $($args)*];
        $crate::__internal_apply_methods_loop!((), this, { $($rest)* })
    }};
    (($($path:tt)+), $this:expr, { ! { $($args:tt)* } $($rest:tt)* }) => {{
        let this = $this;
        let this = $($path)+!{ this, $($args)* };
        $crate::__internal_apply_methods_loop!((), this, { $($rest)* })
    }};
}

/// Utility to apply methods to an object.
///
/// Normally you would chain method calls like this:
///
/// ```rust
/// foo
///     .bar()
///     .qux(5)
///     .corge("yes", "no")
/// ```
///
/// But with `apply_methods!` you can instead do this:
///
/// ```rust
/// apply_methods!(foo, {
///     .bar()
///     .qux(5)
///     .corge("yes", "no")
/// })
/// ```
///
/// In addition to looking nicer, it has another benefit, which is
/// that it supports macros:
///
/// ```rust
/// apply_methods!(foo, {
///     .bar!()
///     .qux!(5)
///     .corge!("yes", "no")
/// })
/// ```
///
/// If you didn't use `apply_methods!` then you would have to write
/// this instead, which is a lot less readable:
///
/// ```rust
/// corge!(qux!(bar!(foo), 5), "yes", "no")
/// ```
///
/// It also supports macro paths:
///
/// ```rust
/// apply_methods!(foo, {
///     .some_crate::bar!()
///     .other_crate::qux!(5)
///     .nested::sub_crate::corge!("yes", "no")
/// })
/// ```
///
/// And it supports specifying the type for method calls:
///
/// ```rust
/// apply_methods!(foo, {
///     .bar::<String>()
///     .qux::<i32, i32>(5)
///     .corge::<Vec<CustomType>>("yes", "no")
/// })
/// ```
///
/// # Creating custom macros
///
/// When using macros inside of `apply_methods!`, the object is
/// always passed as the first argument to the macro:
///
/// ```rust
/// macro_rules! my_macro {
///     ($this:ident, $first:expr, $second:expr) => {
///         ...
///     };
/// }
///
/// // This is the same as doing `my_macro!(foo, 5, 10)`
/// //
/// // That means `$this` is a reference to `foo`
/// apply_methods!(foo, {
///     .my_macro!(5, 10)
/// })
/// ```
///
/// The first argument is *always* an `ident`, regardless of what the object is.
///
/// If the macro doesn't accept any arguments, then it must be written like this,
/// with a trailing comma:
///
/// ```rust
/// macro_rules! my_macro {
///     ($this:ident,) => {
///         ...
///     };
/// }
/// ```
///
/// In addition to `foo!()` macros, you can also use `foo![]` and `foo! {}` macros.
///
/// And the macro can have whatever syntax it wants, because it's a macro:
///
/// ```rust
/// apply_methods!(foo, {
///     .my_macro!(5; 10 => 15)
///
///     .my_macro![5; 10 => 15]
///
///     .my_macro! {
///         foo = 5,
///         bar = 10,
///     }
/// })
/// ```
///
/// # Using `$crate`
///
/// Rust has a limitation where you cannot use `$crate` inside of macros, which means this does not work:
///
/// ```rust
/// apply_methods!(foo, {
///     .$crate::my_macro!(5, 10)
/// })
/// ```
///
/// Instead you can workaround that by doing this:
///
/// ```rust
/// extern crate self as my_crate;
///
/// apply_methods!(foo, {
///     .my_crate::my_macro!(5, 10)
/// })
/// ```
///
/// Alternatively, if you are using the [`html!`](crate::html) or [`svg!`](crate::svg) macros then you can use the [`apply`](DomBuilder::apply) method:
///
/// ```rust
/// html!("div", {
///     .apply(|dom| $crate::my_macro!(dom, 5, 10))
/// })
/// ```
#[macro_export]
macro_rules! apply_methods {
    ($($args:tt)*) => {
        $crate::__internal_apply_methods_loop!((), $($args)*)
    };
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


/// Gives access to the internal DOM node.
///
/// Sometimes you need to access the real DOM node, for example to call
/// DOM methods. You can use `with_node!` to do that:
///
/// ```rust
/// html!("input" => web_sys::HtmlInputElement, {
///     .with_node!(element => {
///         .event(move |_: events::Input| {
///             // `element` is the internal <input> DOM node,
///             // so we can call HtmlInputElement methods
///             let value = element.value_as_number();
///         })
///     })
/// })
/// ```
#[macro_export]
macro_rules! with_node {
    ($this:ident, $name:ident => { $($methods:tt)* }) => {{
        let $name = $crate::DomBuilder::__internal_element(&$this);
        $crate::apply_methods!($this, { $($methods)* })
    }};
}


/// Conditionally runs the methods based on a [`cfg` rule](https://doc.rust-lang.org/reference/conditional-compilation.html#the-cfg-attribute).
///
/// Sometimes you want to run some [`DomBuilder`] methods only in certain situations.
///
/// For example, you might have some client-only code, and some server-only code. So you can do this:
///
/// ```rust
/// html!("div", {
///     // This runs on both the client and server
///     .class("both")
///
///     // This runs ONLY on the client
///     .with_cfg!(feature = "client", {
///         .class("client")
///         .event(...)
///     })
///
///     // This runs ONLY on the server
///     .with_cfg!(feature = "server", {
///         .class("server")
///     })
/// })
/// ```
///
/// Then when you compile your program, you can set the feature flags by using
/// `--features client` or `--features server`.
///
/// If the `with_cfg!` doesn't match, then the code is completely removed, so it
/// has no performance cost.
///
/// You can create whatever `features` you want, you aren't limited to only
/// `client` and `server`.
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


/// Attaches a shadow root to a [`DomBuilder`].
///
/// The first argument is the [shadow root mode](https://developer.mozilla.org/en-US/docs/Web/API/ShadowRoot/mode) ([`ShadowRootMode::Open`](web_sys::ShadowRootMode::Open) or [`ShadowRootMode::Closed`](web_sys::ShadowRootMode::Closed)).
///
/// The second argument is a block of method calls. Inside of the block you can use [`DomBuilder<web_sys::ShadowRoot>`] methods:
///
/// ```rust
/// use web_sys::ShadowRootMode;
///
/// html!("div", {
///     .shadow_root!(ShadowRootMode::Open, {
///         .child(...)
///         .child_signal(...)
///         .children_signal_vec(...)
///     })
/// })
/// ```
///
/// The method calls are applied to the shadow root, not the parent [`DomBuilder`].
///
/// The block uses the [`apply_methods!`] macro, see the docs for [`apply_methods!`] for more details.
#[macro_export]
macro_rules! shadow_root {
    ($this:ident, $mode:expr => { $($methods:tt)* }) => {{
        let shadow = $crate::DomBuilder::__internal_shadow_root(&$this, $mode);
        let shadow = $crate::apply_methods!(shadow, { $($methods)* });
        $crate::DomBuilder::__internal_transfer_callbacks($this, shadow)
    }};
}


/// Creates an HTML [`Dom`] node.
///
/// The first argument is the [HTML tag](https://developer.mozilla.org/en-US/docs/Web/HTML/Element), and the second argument is a block of method calls.
///
/// Inside of the block you can use [`DomBuilder<web_sys::HtmlElement>`] methods:
///
/// ```rust
/// html!("div", {
///     .class("foo")
///     .style("color", "green")
///     .style_signal("width", ...)
/// })
/// ```
///
/// The block uses the [`apply_methods!`] macro, see the docs for [`apply_methods!`] for more details.
///
/// You can also specify the static type of the HTML element:
///
/// ```rust
/// html!("div" => web_sys::HtmlDivElement, {
///     ...
/// })
/// ```
///
/// If you don't specify a type, it defaults to [`web_sys::HtmlElement`].
#[macro_export]
macro_rules! html {
    ($($args:tt)+) => {
        $crate::__internal_builder!($crate::__internal::HtmlElement, new_html, $($args)+)
    };
}


/// Creates an SVG [`Dom`] node.
///
/// The first argument is the [SVG tag](https://developer.mozilla.org/en-US/docs/Web/SVG/Element), and the second argument is a block of method calls.
///
/// Inside of the block you can use [`DomBuilder<web_sys::SvgElement>`] methods:
///
/// ```rust
/// svg!("line", {
///     .class("foo")
///     .attr("x1", "5")
///     .attr_signal("x2", ...)
/// })
/// ```
///
/// The block uses the [`apply_methods!`] macro, see the docs for [`apply_methods!`] for more details.
///
/// You can also specify the static type of the SVG element:
///
/// ```rust
/// html!("line" => web_sys::SvgLineElement, {
///     ...
/// })
/// ```
///
/// If you don't specify a type, it defaults to [`web_sys::SvgElement`].
#[macro_export]
macro_rules! svg {
    ($($args:tt)+) => {
        $crate::__internal_builder!($crate::__internal::SvgElement, new_svg, $($args)+)
    };
}


/// Converts an existing DOM node into a dominator [`Dom`] node.
///
/// This is useful for applying [`DomBuilder`] methods to an already-existing DOM node (for example a third-party library).
///
/// The first argument is the DOM node, and the second argument is a block of method calls.
///
/// Inside of the block you can use [`DomBuilder`] methods:
///
/// ```rust
/// dom_builder!(my_dom_node, {
///     .class("foo")
///     .style("color", "green")
///     .style_signal("width", ...)
/// })
/// ```
///
/// The block uses the [`apply_methods!`] macro, see the docs for [`apply_methods!`] for more details.
#[macro_export]
macro_rules! dom_builder {
    ($node:expr, { $($methods:tt)* }) => {{
        let builder = $crate::DomBuilder::new($node);
        let output = $crate::apply_methods!(builder, { $($methods)* });
        $crate::DomBuilder::into_dom(output)
    }};
}


/// Creates a global CSS stylesheet.
///
/// The stylesheet applies to the entire page, it is the same as importing a `.css` file,
/// except the stylesheet is created entirely with Rust code.
///
/// The first argument is the CSS selector, and the second argument is a block of method calls.
///
/// Inside of the block you can use [`StylesheetBuilder`] methods:
///
/// ```rust
/// stylesheet!("div.foo > span:nth-child(5):hover", {
///     .style("color", "green")
///     .style("background-color", "blue")
///     .style_signal("width", ...)
/// });
/// ```
///
/// The block uses the [`apply_methods!`] macro, see the docs for [`apply_methods!`] for more details.
#[macro_export]
macro_rules! stylesheet {
    ($rule:expr) => {
        $crate::stylesheet!($rule, {})
    };
    ($rule:expr, { $($methods:tt)* }) => {
        $crate::StylesheetBuilder::__internal_done($crate::apply_methods!($crate::StylesheetBuilder::__internal_new($rule), { $($methods)* }))
    };
}


/// Creates a locally scoped CSS stylesheet.
///
/// Normally CSS is global, which means you can accidentally create name collisions by using
/// the same class name multiple times.
///
/// However, if you use the `class!` macro, then it is impossible to have name collisions,
/// because the CSS is locally scoped.
///
/// This makes it a lot easier to create self-contained components, so you should prefer
/// to use `class!` as much as you can.
///
/// The `class!` macro accepts a block of method calls. Inside of the block you can use [`ClassBuilder`] methods:
///
/// ```rust
/// class! {
///     .style("color", "green")
///     .style("background-color", "blue")
///     .style_signal("width", ...)
/// }
/// ```
///
/// The block uses the [`apply_methods!`] macro, see the docs for [`apply_methods!`] for more details.
///
/// The `class!` macro returns a `String`, which is a unique class name. You can then assign that class
/// name to a [`DomBuilder`]:
///
/// ```rust
/// use once_cell::sync::Lazy;
///
/// // This uses a `static` so that it only creates the `class!` a single time.
/// //
/// // If it used `let` then it would create the same `class!` multiple times.
/// static MY_CLASS: Lazy<String> = Lazy::new(|| class! {
///     .style("color", "green")
///     .style("background-color", "blue")
///     .style_signal("width", ...)
/// });
///
/// html!("div", {
///     .class(&*MY_CLASS)
/// })
/// ```
///
/// The class is locally scoped, which means it cannot conflict with any other classes, the only way
/// to access the class is by using the `MY_CLASS` variable.
///
/// Because it is a normal Rust variable, it follows the normal Rust scoping rules. By default the variable
/// can only be acccessed within that module.
///
/// But you can use `pub` to export it so it can be used by other modules:
///
/// ```rust
/// pub static MY_CLASS: Lazy<String> = Lazy::new(|| class! {
///     ...
/// });
/// ```
///
/// Or you can use `pub(crate)` so that it can only be accessed within your crate.
#[macro_export]
macro_rules! class {
    (#![prefix = $name:literal] $($methods:tt)*) => {{
        $crate::ClassBuilder::__internal_done($crate::apply_methods!($crate::ClassBuilder::__internal_new(Some($name)), { $($methods)* }))
    }};
    ($($methods:tt)*) => {{
        $crate::ClassBuilder::__internal_done($crate::apply_methods!($crate::ClassBuilder::__internal_new(None), { $($methods)* }))
    }};
}


/// Adds a pseudo rule to a [`class!`] stylesheet.
///
/// A pseudo rule is either a [pseudo class](https://developer.mozilla.org/en-US/docs/Web/CSS/Pseudo-classes) or a [pseudo element](https://developer.mozilla.org/en-US/docs/Web/CSS/Pseudo-elements).
///
/// The first argument is a string, or an array of strings.
///
/// The second argument is a block of method calls. Inside of the block you can use [`ClassBuilder`] methods:
///
/// ```rust
/// class! {
///     .pseudo(":hover", {
///         .style("color", "green")
///         .style("background-color", "blue")
///         .style_signal("width", ...)
///     })
/// }
/// ```
///
/// The block uses the [`apply_methods!`] macro, see the docs for [`apply_methods!`] for more details.
///
/// If the first argument is an array of strings, it will try each pseudo rule in order until it finds one that works.
///
/// This is useful for using browser prefixes:
///
/// ```rust
/// class! {
///     .pseudo([":any-link", ":-webkit-any-link"], {
///         ...
///     })
/// }
/// ```
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

// TODO move into gloo ?
/// Helper utility that calls [`clone`](std::clone::Clone::clone).
///
/// When you use event listeners, you often need to [`clone`](std::clone::Clone::clone) some state:
///
/// ```rust
/// let app = app.clone();
/// let state = state.clone();
/// let other_state = other_state.clone();
///
/// html!("div", {
///     .event(move |_: events::Click| {
///         // Use app, state, and other_state
///     })
/// })
/// ```
///
/// You can achieve the same thing by using the `clone!` macro instead:
///
/// ```rust
/// html!("div", {
///     .event(clone!(app, state, other_state => move |_: events::Click| {
///         // Use app, state, and other_state
///     }))
/// })
/// ```
#[macro_export]
macro_rules! clone {
    ($($input:tt)*) => { $crate::__internal_clone_split!((), $($input)*) };
}


/// A convenient shorthand for multiple `.attr(...)` calls.
///
/// Instead of writing this...
///
/// ```rust
/// html!("div", {
///     .attr("foo", "bar")
///     .attr("qux", "corge")
///     .attr("yes", "no")
/// })
/// ```
///
/// ...you can instead write this:
///
/// ```rust
/// html!("div", {
///     .attrs! {
///         foo: "bar",
///         qux: "corge",
///         yes: "no",
///     }
/// })
/// ```
#[macro_export]
macro_rules! attrs {
    ($this:ident, $($name:ident: $value:expr),*,) => {
        $crate::attrs!($this, $($name: $value),*)
    };
    ($this:ident, $($name:ident: $value:expr),*) => {
        $crate::apply_methods!($this, {
            $(.attr(::std::stringify!($name), $value))*
        })
    };
}


/// A convenient shorthand for multiple `.prop(...)` calls.
///
/// Instead of writing this...
///
/// ```rust
/// html!("div", {
///     .prop("foo", "bar")
///     .prop("qux", "corge")
///     .prop("yes", "no")
/// })
/// ```
///
/// ...you can instead write this:
///
/// ```rust
/// html!("div", {
///     .props! {
///         foo: "bar",
///         qux: "corge",
///         yes: "no",
///     }
/// })
/// ```
#[macro_export]
macro_rules! props {
    ($this:ident, $($name:ident: $value:expr),*,) => {
        $crate::props!($this, $($name: $value),*)
    };
    ($this:ident, $($name:ident: $value:expr),*) => {
        $crate::apply_methods!($this, {
            $(.prop(::std::stringify!($name), $value))*
        })
    };
}
