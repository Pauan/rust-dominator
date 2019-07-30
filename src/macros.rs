#[doc(hidden)]
#[macro_export]
macro_rules! __internal_builder_method {
    ($this:expr,) => {
        $this
    };
    ($this:expr, .$name:ident!($($args:tt)*) $($rest:tt)*) => {
        $crate::__internal_builder_method!($name!($this, $($args)*), $($rest)*)
    };
    ($this:expr, .$name:ident($($args:expr),*) $($rest:tt)*) => {
        $crate::__internal_builder_method!($this.$name($($args),*), $($rest)*)
    };
}


#[doc(hidden)]
#[macro_export]
macro_rules! __internal_builder {
    ($default:ty, $name:ident => $make:expr, $kind:expr) => {
        $crate::__internal_builder!($default, $name => $make, $kind => $default, {})
    };
    ($default:ty, $name:ident => $make:expr, $kind:expr, $($rest:tt)*) => {
        $crate::__internal_builder!($default, $name => $make, $kind => $default, $($rest)*)
    };
    ($default:ty, $name:ident => $make:expr, $kind:expr => $t:ty) => {
        $crate::__internal_builder!($default, $name => $make, $kind => $t, {})
    };
    ($default:ty, $name:ident => $make:expr, $kind:expr => $t:ty, { $($methods:tt)* }) => {{
        let a: $t = {
            let $name = $kind;
            $make
        };
        let b = $crate::DomBuilder::new(a);
        let c = $crate::__internal_builder_method!(b, $($methods)*);
        $crate::DomBuilder::into_dom(c)
    }};
}


#[macro_export]
macro_rules! with_node {
    ($this:expr, $name:ident => { $($methods:tt)* }) => {{
        let $name = $crate::DomBuilder::__internal_element(&$this);
        $crate::__internal_builder_method!($this, $($methods)*)
    }};
}


#[macro_export]
macro_rules! html {
    ($($args:tt)+) => {
        $crate::__internal_builder!($crate::HtmlElement, kind => $crate::create_element(kind), $($args)+)
    };
}


#[macro_export]
macro_rules! svg {
    ($($args:tt)+) => {
        $crate::__internal_builder!($crate::SvgElement, kind => $crate::create_element_ns(kind, $crate::SVG_NAMESPACE), $($args)+)
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
