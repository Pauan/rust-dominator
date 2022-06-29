use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, intern};
use web_sys::{HtmlElement, Element, Node, Window, History, Document, Text, Comment, DomTokenList, CssStyleSheet, CssStyleDeclaration, HtmlStyleElement, CssStyleRule, EventTarget};


#[wasm_bindgen(inline_js = "
    export function set_property(obj, name, value) { obj[name] = value; }
")]
extern "C" {
    // TODO move this into wasm-bindgen or gloo or something
    // TODO maybe use Object for obj ?
    pub(crate) fn set_property(obj: &JsValue, name: &str, value: &JsValue);
}


thread_local! {
    static WINDOW: Window = web_sys::window().unwrap_throw();
    static DOCUMENT: Document = WINDOW.with(|w| w.document().unwrap_throw());
    static HISTORY: History = WINDOW.with(|w| w.history().unwrap_throw());
}

pub(crate) fn window_event_target() -> EventTarget {
    // TODO avoid the clone somehow ?
    WINDOW.with(|w| w.clone().into())
}

pub(crate) fn body() -> HtmlElement {
    DOCUMENT.with(|d| d.body().unwrap_throw())
}

pub(crate) fn ready_state() -> String {
    DOCUMENT.with(|d| d.ready_state())
}

pub(crate) fn current_url() -> String {
    WINDOW.with(|w| w.location().href().unwrap_throw())
}

pub(crate) fn go_to_url(url: &str) {
    HISTORY.with(|h| {
        h.push_state_with_url(&JsValue::NULL, "", Some(url)).unwrap_throw();
    });
}

pub(crate) fn create_stylesheet() -> CssStyleSheet {
    DOCUMENT.with(|document| {
        // TODO use createElementNS ?
        // TODO use dyn_into ?
        let e: HtmlStyleElement = document.create_element("style").unwrap_throw().unchecked_into();
        e.set_type("text/css");
        append_child(&document.head().unwrap_throw(), &e);
        // TODO use dyn_into ?
        e.sheet().unwrap_throw().unchecked_into()
    })
}

pub(crate) fn make_style_rule(sheet: &CssStyleSheet, selector: &str) -> Result<CssStyleRule, JsValue> {
    let rules = sheet.css_rules().unwrap_throw();
    let length = rules.length();
    // TODO don't return u32 ?
    sheet.insert_rule_with_index(&format!("{}{{}}", selector), length)?;
    // TODO use dyn_into ?
    Ok(rules.get(length).unwrap_throw().unchecked_into())
}


pub(crate) fn get_element_by_id(id: &str) -> Element {
    DOCUMENT.with(|d| d.get_element_by_id(id).unwrap_throw())
}

pub(crate) fn create_element(name: &str) -> Element {
    DOCUMENT.with(|d| d.create_element(name).unwrap_throw())
}

pub(crate) fn create_element_ns(namespace: &str, name: &str) -> Element {
    DOCUMENT.with(|d| d.create_element_ns(Some(namespace), name).unwrap_throw())
}

pub(crate) fn create_text_node(value: &str) -> Text {
    DOCUMENT.with(|d| d.create_text_node(value))
}

pub(crate) fn set_text(elem: &Text, value: &str) {
    // http://jsperf.com/textnode-performance
    elem.set_data(value);
}

pub(crate) fn create_comment(value: &str) -> Comment {
    DOCUMENT.with(|d| d.create_comment(value))
}

#[inline]
pub(crate) fn create_empty_node() -> Node {
    // TODO is there a better way of doing this ?
    create_comment(intern("")).into()
}

// TODO check that the attribute *actually* was changed
pub(crate) fn set_attribute(elem: &Element, key: &str, value: &str) {
    elem.set_attribute(key, value).unwrap_throw();
}

pub(crate) fn set_attribute_ns(elem: &Element, namespace: &str, key: &str, value: &str) {
    elem.set_attribute_ns(Some(namespace), key, value).unwrap_throw();
}

pub(crate) fn remove_attribute(elem: &Element, key: &str) {
    elem.remove_attribute(key).unwrap_throw();
}

pub(crate) fn remove_attribute_ns(elem: &Element, namespace: &str, key: &str) {
    elem.remove_attribute_ns(Some(namespace), key).unwrap_throw();
}

pub(crate) fn add_class(classes: &DomTokenList, value: &str) {
    classes.add_1(value).unwrap_throw();
}

pub(crate) fn remove_class(classes: &DomTokenList, value: &str) {
    classes.remove_1(value).unwrap_throw();
}

pub(crate) fn get_style(style: &CssStyleDeclaration, name: &str) -> String {
    style.get_property_value(name).unwrap_throw()
}

pub(crate) fn remove_style(style: &CssStyleDeclaration, name: &str) {
    // TODO don't return String ?
    style.remove_property(name).unwrap_throw();
}

pub(crate) fn set_style(style: &CssStyleDeclaration, name: &str, value: &str, important: bool) {
    let priority = if important { intern("important") } else { intern("") };
    style.set_property_with_priority(name, value, priority).unwrap_throw();
}

pub(crate) fn insert_child_before(parent: &Node, child: &Node, other: &Node) {
    // TODO don't return Node ?
    parent.insert_before(child, Some(other)).unwrap_throw();
}

pub(crate) fn replace_child(parent: &Node, child: &Node, other: &Node) {
    // TODO don't return Node ?
    parent.replace_child(child, other).unwrap_throw();
}

pub(crate) fn append_child(parent: &Node, child: &Node) {
    // TODO don't return Node ?
    parent.append_child(child).unwrap_throw();
}

pub(crate) fn remove_child(parent: &Node, child: &Node) {
    // TODO don't return Node ?
    parent.remove_child(child).unwrap_throw();
}

pub(crate) fn focus(elem: &HtmlElement) {
    elem.focus().unwrap_throw();
}

pub(crate) fn blur(elem: &HtmlElement) {
    elem.blur().unwrap_throw();
}
