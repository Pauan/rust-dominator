#[macro_export]
macro_rules! html {
    ($kind:expr => $t:ty) => {
        html!($kind => $t, {})
    };
    ($kind:expr => $t:ty, { $( $name:ident( $( $args:expr ),* ); )* }) => {{
        let a: $crate::HtmlBuilder<$t> = $crate::HtmlBuilder::new($kind)$(.$name($($args),*))*;
        let b: $crate::Dom = a.into();
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


// TODO move into stdweb
#[macro_export]
macro_rules! clone {
    ({$($x:ident),+} $y:expr) => {{
        $(let $x = $x.clone();)+
        $y
    }};
}
