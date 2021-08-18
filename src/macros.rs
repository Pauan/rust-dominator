/// Allows the application of methods to a type using the standard `dominator` macro syntax. Used internally by
/// most of the other macros [`html!`](crate::html!), [`with_node!`](crate::with_node!), [`with_cfg!`](crate::with_cfg!),
/// [`shadow_root!`](crate::shadow_root!), [`dom_builder!`](crate::dom_builder!), [`stylesheet!`](crate::stylesheet!),
/// [`class!`](crate::class!). It puts each method call in a separate statement: this is to ensure that a
/// [lock](https://doc.rust-lang.org/std/sync/struct.RwLock.html) created inside one method call will not extend
/// to the rest of the method calls.
/// Since [`Mutable<T>`](https://docs.rs/futures-signals/*/futures_signals/signal/struct.Mutable.html),
/// [`MutableVec<T>`](https://docs.rs/futures-signals/*/futures_signals/signal_vec/struct.MutableVec.html), or
/// [`MutableBTreeMap<K, V>`](https://docs.rs/futures-signals/*/futures_signals/signal_map/struct.MutableBTreeMap.html) all use
/// [RwLock](https://doc.rust-lang.org/std/sync/struct.RwLock.html) internally this is important for subsequent method calls
/// that may want to access the same data.
/// It also accepts some of the other macros [`with_node!`](crate::with_node!), [`with_cfg!`](crate::with_cfg!),
/// [`shadow_root!`](crate::shadow_root!), [`pseudo!`](crate::pseudo) and will elide the first `this:expr` parameter
/// in the macro calls.
///
/// ```
/// let body = web_sys::window()
///     .unwrap()
///     .document()
///     .unwrap()
///     .body()
///     .unwrap();
///
/// let builder = DomBuilder::new(body);
///
/// apply_methods!(builder, {
///     .with_node!(node => {
///         .event(move |_: events::Click| node.set_inner_text("Goodbye"))
///         .text("Hello world")
///     })
///     .class("my-class")
/// });
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

/// Provides owned access to the internal element of a [`dom_builder!`](crate::dom_builder!) and is used for
/// passing the element into other builder methods. Runs before the element is inserted into the DOM.
/// Typically used within [`html!`](crate::html!) where the first builder term is elided.
/// ```
/// html!("div", {
///     .with_node!(div => {
///         // allows access to the underlying 'div' HtmlElement
///         .event(move |_: events::Click| div.set_inner_text("Goodbye"))
///     })
///     .text("Hello world")
/// });
/// ```
/// It can also be used directly with [dom_builder!](crate::dom_builder!):
/// ```
/// let body = web_sys::window()
///     .unwrap()
///     .document()
///     .unwrap()
///     .body()
///     .unwrap();
///
/// let builder = DomBuilder::new(body);
///
/// with_node!(builder, body => {
///     .event(move |_: events::Click| body.set_inner_text("Goodbye"))
///     .text("Hello world")
/// });
/// ```
#[macro_export]
macro_rules! with_node {
    ($this:ident, $name:ident => { $($methods:tt)* }) => {{
        let $name = $crate::DomBuilder::__internal_element(&$this);
        $crate::apply_methods!($this, { $($methods)* })
    }};
}

/// Used to apply methods to a builder based upon a standard rust `#[cfg(predicate)]`.
/// Typically used within one of the other builder macros where the first builder term is elided.
/// If used with the [rollup-plugin-rust](https://github.com/wasm-tool/rollup-plugin-rust#build-options) the
/// required `cargo build` arguments are added using `cargoArgs: ["--features", "foo"]`.
/// ```
/// html!("div", {
///     .with_cfg!(feature = "foo", {
///         .text("Using the foo feature")
///     })
/// });
/// ```
/// It can also be used directly with any of the builders.
/// ```
/// let body = web_sys::window()
///     .unwrap()
///     .document()
///     .unwrap()
///     .body()
///     .unwrap();
///
/// let builder = DomBuilder::new(body);
///
/// with_cfg!(builder, feature = "foo", {
///     .text("Using the foo feature")
/// });
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

/// Attaches DOM elements to the [ShadowRoot](https://developer.mozilla.org/en-US/docs/Web/API/ShadowRoot) of
/// a DOM element. The internal element type of the macro is [`ShadowRoot`](https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.ShadowRoot.html)
/// and it requires [`ShadowRootMode`](https://rustwasm.github.io/wasm-bindgen/api/web_sys/enum.ShadowRootMode.html) as a parameter.
/// ```
/// shadow_root!(dom_builder, shadow_root_mode => { .dom_builder_methods })
/// ```
/// Typically used within the [`html!`](crate::html!) macro where the [`DomBuilder<A>`](crate::DomBuilder) is elided:
/// ```
/// html!("div", {
///     .shadow_root!(web_sys::ShadowRootMode::Open => {
///         .text("Shadow dom")
///     })
/// });
/// ```
/// It can also be used directly with [`DomBuilder<A>`](crate::DomBuilder):
/// ```
/// let body = web_sys::window()
///     .unwrap()
///     .document()
///     .unwrap()
///     .body()
///     .unwrap();
///
/// let builder = DomBuilder::new(body);
///
/// shadow_root!(builder, web_sys::ShadowRootMode::Open => {
///     .child(html!("div", {
///         .text("Shadow dom")
///     }))
/// });
/// ```
#[macro_export]
macro_rules! shadow_root {
    ($this:ident, $mode:expr => { $($methods:tt)* }) => {{
        let shadow = $crate::DomBuilder::__internal_shadow_root(&$this, $mode);
        let shadow = $crate::apply_methods!(shadow, { $($methods)* });
        $crate::DomBuilder::__internal_transfer_callbacks($this, shadow)
    }};
}

/// Used to build a [`Dom`](crate::dom::Dom) and is a wrapper over [`DomBuilder<A>`](crate::dom::DomBuilder).
/// By default the macro is internally typed to [`HtmlElement`](https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.HtmlElement.html).
/// Note if you want to generate [SVG](http://www.w3.org/2000/svg) make sure to use the [svg!](crate::svg!) macro.
/// ```
/// html!("html_tag", { .dom_builder_methods })
/// ```
/// ```
/// // <div></div>
/// html!("div");
///
/// // <my-tag></my-tag>
/// html!("my-tag");
///
/// // <div>Hello world</div>
/// html!("div", {
///     .text("Hello world")
/// });
///
/// // <div class="my-class">Hello world</div>
/// html!("div", {
///      .text("Hello world")
///      .class("my-class")
///  });
/// ```
/// It can be typed to any [element](https://rustwasm.github.io/wasm-bindgen/api/web_sys/index.html) that
/// `impl` [JsCast](https://rustwasm.github.io/wasm-bindgen/api/wasm_bindgen/trait.JsCast.html#).
/// ```
/// html!("html_tag" => internal_element_type , { .dom_builder_methods })
/// ```
/// e.g. [HtmlInputElement](https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.HtmlInputElement.html)
/// ```
/// // <input placeholder="type here">
/// html!("input" => HtmlInputElement, {
///     .attr("placeholder", "type here")
/// });
/// ```
/// Why would you want to know the type? Well once you start using some of the [DomBuilder](crate::dom::DomBuilder) methods
/// knowing the html element type allows you to call its associated methods:
/// ```
/// html!("input" => HtmlInputElement, {
///     .before_inserted(|input| {
///         // .value() is a method on HtmlInputElement, but not on HtmlElement
///         if input.value().is_empty() {
///             input.set_value("Hello")
///         }
///     })
/// });
/// ```
#[macro_export]
macro_rules! html {
    ($($args:tt)+) => {
        $crate::__internal_builder!($crate::__internal::HtmlElement, new_html, $($args)+)
    };
}

/// Used to create SVG elements. Exactly like the [`html!`](crate::html!) macro in that it wraps [`DomBuilder<A>`](crate::dom::DomBuilder)
/// but it creates elements correctly namespaced to [SVG](http://www.w3.org/2000/svg). The default internal element
/// type is [`SvgElement`](https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.SvgElement.html)
///
/// ```
/// // <svg>
/// //   <rect width="100px" height="100px"></rect>
/// // </svg>
/// svg!("svg", {
///     .child(svg!("rect" => SvgRectElement, {
///         .attr("width", "100px")
///         .attr("height", "100px")
///     }))
/// });
/// ```
#[macro_export]
macro_rules! svg {
    ($($args:tt)+) => {
        $crate::__internal_builder!($crate::__internal::SvgElement, new_svg, $($args)+)
    };
}

/// Used to wrap an existing DOM node in a [`DomBuilder<A>`](crate::dom::DomBuilder)
/// where the node being wrapped must [`impl AsRef<web_sys::Element>`](https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.Element.html).
/// ```
/// dom_builder!(node_to_wrap, { .dom_builder_methods })
/// ```

/// ```
/// let body = web_sys::window()
///     .unwrap()
///     .document()
///     .unwrap()
///     .body()
///     .unwrap();
///
/// dom_builder!(body, {
///     .before_inserted(|body| body.set_inner_text("Hello world"))
/// });
/// ```
#[macro_export]
macro_rules! dom_builder {
    ($node:expr, { $($methods:tt)* }) => {{
        let builder = $crate::DomBuilder::new($node);
        let output = $crate::apply_methods!(builder, { $($methods)* });
        $crate::DomBuilder::into_dom(output)
    }};
}

/// Creates a global CSS stylesheet: similar to creating a `.css` file. It wraps [`StylesheetBuilder`](crate::StylesheetBuilder),
/// the first argument is the element selector(s) that needs to `impl` [`MultiStr`](crate::traits::MultiStr) and then
/// any of the [`StylesheetBuilder`](crate::StylesheetBuilder) methods.
/// ```
/// stylesheet("tag_selector(s)", { .stylesheet_builder_methods })
/// ```
/// It is not often used since [class!](crate::class!) is superior but its useful for changing the html and body elements:
/// ```
/// stylesheet!("html, body", {
///     .style("margin", "0px")
/// })
/// ```
/// Or applying styles to everything:
/// ```
/// stylesheet!("*", {
///     .style("box-sizing", "border-box")
/// })
/// ```
#[macro_export]
macro_rules! stylesheet {
    ($rule:expr) => {
        $crate::stylesheet!($rule, {})
    };
    ($rule:expr, { $($methods:tt)* }) => {
        $crate::StylesheetBuilder::__internal_done($crate::apply_methods!($crate::StylesheetBuilder::__internal_new($rule), { $($methods)* }))
    };
}

/// Creates a unique auto-generated CSS classname and injects it into [`document.styleSheets`](https://developer.mozilla.org/en-US/docs/Web/API/Document/styleSheets).
/// It wraps [`ClassBuilder`](crate::ClassBuilder) and takes any of its methods.
/// ```
/// let padding = Mutable::new(10);
///
/// let class = class! {
///     .style("display", "block")
///     .style_signal("padding", padding.signal_cloned().map(|v| format!("{}px", v)))
/// };
///
/// html!("button", {
///     .class(class)
///     .event(move |_: events::Click| {
///         let mut s = padding.lock_mut();
///         *s += 10;
///     })
///     .text("Push")
/// });
/// ```
/// It is often declared as `static` (when not using `.style_signal`) so it is only made once:
/// ```
/// use once_cell::sync::Lazy;
///
/// static BUTTON_CLASS: Lazy<String> = Lazy::new(|| {
///     class! {
///         .style("padding", "20px")
///     }
/// });
///
/// html!("button", {
///     .class(&*BUTTON_CLASS)
/// });
/// ```
#[macro_export]
macro_rules! class {
    ($($methods:tt)*) => {{
        $crate::ClassBuilder::__internal_done($crate::apply_methods!($crate::ClassBuilder::__internal_new(), { $($methods)* }))
    }};
}

/// Used to generate [pseudo classes and elements](https://developer.mozilla.org/en-US/docs/Learn/CSS/Building_blocks/Selectors/Pseudo-classes_and_pseudo-elements).
/// Usually called from within the [`class!`](crate::class!) macro:
/// ```
/// class!(
///     .style("background-color", "black")
///     .pseudo!(":nth-child(1)", {
///         .style("background-color", "blue")
///     })
///     .pseudo!("::after", {
///         .style("background-color", "red")
///     })
/// );
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
// TODO move into gloo ?

/// Used as a shorthand syntax for calling `.clone()` when parsing variables into functions. Due to
/// `'static` requirements of many of the web apis `move` and `clone` are frequently required.
/// ```
/// clone!(clone_item_1, clone_item_2, clone_item_3 => { expression using the clones })
/// ```
/// Where the `clone_item_x` shadow existing variable names.
/// So instead of writing:
/// ```
/// let text = Mutable::new("text");
/// let other_text = Mutable::new("other text");
/// html!("div", {
///     .event({
///         let text = text.clone();
///         let other_text = text.clone();
///         move |_: events::Click| {
///         text.set("clicked");
///         other_text.set("clicked")
///     }})
///     .event({
///         let other_text = other_text.clone();
///         move |_: events::Focus| {
///         text.set("focused");
///         other_text.set("focused")
///     }})
///     .text_signal(other_text.signal_cloned())
/// });
/// ```
/// `clone!` can make a cleaner syntax sharing the same variable names:
/// ```
/// let text = Mutable::new("text");
/// let other_text = Mutable::new("other text");
/// html!("div", {
///     .event(clone!(text, other_text => move |_: events::Click| {
///         text.set("clicked");
///         other_text.set("clicked")
///     }))
///     .event(clone!(other_text => move |_: events::Focus| {
///         text.set("focused");
///         other_text.set("focused")
///     }))
///     .text_signal(other_text.signal_cloned())
/// });
/// ```
#[macro_export]
macro_rules! clone {
    ($($x:ident),+ => $y:expr) => {{
        $(let $x = $x.clone();)+
        $y
    }};
}
