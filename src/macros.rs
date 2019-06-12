#[doc(hidden)]
#[macro_export]
macro_rules! __internal_builder_method {
    ($this:expr,) => {
        $this
    };
    ($this:expr, .$name:ident!($($args:expr),*) $($rest:tt)*) => {
        $crate::__internal_builder_method!($name!($this, $($args),*), $($rest)*)
    };
    ($this:expr, .$name:ident($($args:expr),*) $($rest:tt)*) => {
        $crate::__internal_builder_method!($this.$name($($args),*), $($rest)*)
    };
}


#[macro_export]
macro_rules! builder {
    ($namespace:expr, $default:ty, $kind:expr => $t:ty) => {
        $crate::builder!($namespace, $default, $kind => $t, {})
    };
    ($namespace:expr, $default:ty, $kind:expr => $t:ty, { $($methods:tt)* }) => {{
        let a: $t = $crate::create_element_ns($kind, $namespace);
        let b = $crate::DomBuilder::new(a);
        let c = $crate::__internal_builder_method!(b, $($methods)*);
        $crate::DomBuilder::into_dom(c)
    }};

    ($namespace:expr, $default:ty, $kind:expr) => {
        $crate::builder!($namespace, $default, $kind => $default)
    };
    ($namespace:expr, $default:ty, $kind:expr, { $($methods:tt)* }) => {
        $crate::builder!($namespace, $default, $kind => $default, { $($methods)* })
    };
}


#[macro_export]
macro_rules! html {
    ($($args:tt)+) => {
        $crate::builder!($crate::HTML_NAMESPACE, $crate::HtmlElement, $($args)+)
    };
}


#[macro_export]
macro_rules! svg {
    ($($args:tt)+) => {
        $crate::builder!($crate::SVG_NAMESPACE, $crate::SvgElement, $($args)+)
    };
}


#[macro_export]
macro_rules! stylesheet {
    ($rule:expr) => {
        $crate::stylesheet!($rule, {})
    };
    ($rule:expr, { $($methods:tt)* }) => {{
        let a = $crate::StylesheetBuilder::new($rule);
        $crate::__internal_builder_method!(a, $($methods)*).done()
    }};
}


#[macro_export]
macro_rules! class {
    ($($methods:tt)*) => {{
        let a = $crate::ClassBuilder::new();
        $crate::__internal_builder_method!(a, $($methods)*).done()
    }};
}


// TODO this is pretty inefficient, it iterates over the token tree one token at a time
#[doc(hidden)]
#[macro_export]
macro_rules! __internal_clone_split {
    (($($x:ident)*), $t:ident => $y:expr) => {{
        $(let $x = ::std::clone::Clone::clone(&$x);)*
        let $t = ::std::clone::Clone::clone(&$t);
        $y
    }};
    (($($x:ident)*), $t:ident, $($after:tt)*) => {
        $crate::__internal_clone_split!(($($x)* $t), $($after)*)
    };
}

// TODO move into gloo ?
#[macro_export]
macro_rules! clone {
    ($($input:tt)*) => { $crate::__internal_clone_split!((), $($input)*) };
}
