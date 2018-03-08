#[macro_export]
macro_rules! html {
    ($kind:expr => $t:ty) => {
        html!($kind => $t, {})
    };
    ($kind:expr => $t:ty, { $( $name:ident( $( $args:expr ),* ); )* }) => {{
        let a: $crate::HtmlBuilder<$t> = $crate::HtmlBuilder::new($kind)$(.$name($($args),*))*;
        let b: $crate::Dom = ::std::convert::Into::into(a);
        b
    }};

    ($kind:expr) => {
        // TODO need better hygiene for HtmlElement
        html!($kind => HtmlElement)
    };
    ($kind:expr, { $( $name:ident( $( $args:expr ),* ); )* }) => {{
        // TODO need better hygiene for HtmlElement
        html!($kind => HtmlElement, { $( $name( $( $args ),* ); )* })
    }};
}


#[macro_export]
macro_rules! stylesheet {
    ($rule:expr) => {
        stylesheet!($rule, {})
    };
    ($rule:expr, { $( $name:ident( $( $args:expr ),* ); )* }) => {{
        $crate::StylesheetBuilder::new($rule)$(.$name($($args),*))*.done()
    }};
}


#[macro_export]
macro_rules! class {
    ($( $name:ident( $( $args:expr ),* ); )*) => {{
        $crate::ClassBuilder::new()$(.$name($($args),*))*.done()
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
        __internal_clone_split!(($($x)* $t), $($after)*)
    };
}

// TODO move into stdweb ?
#[macro_export]
macro_rules! clone {
    ($($input:tt)*) => { __internal_clone_split!((), $($input)*) };
}
