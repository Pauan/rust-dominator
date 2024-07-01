use wasm_bindgen::prelude::*;
use wasm_bindgen::{JsCast, intern};
use js_sys::Reflect;
use web_sys::{HtmlElement, Element, Node, Window, History, Document, Text, Comment, DomTokenList, CssStyleSheet, CssStyleDeclaration, HtmlStyleElement, CssRule};
use crate::utils::UnwrapJsExt;


// TODO move this into wasm-bindgen or gloo or something
// TODO maybe use Object for obj ?
#[track_caller]
pub(crate) fn set_property(obj: &JsValue, name: &str, value: &JsValue) {
    Reflect::set(obj, &JsValue::from(name), value).unwrap_js();
}


thread_local! {
    pub static WINDOW: Window = web_sys::window().unwrap_throw();
    static DOCUMENT: Document = WINDOW.with(|w| w.document().unwrap_throw());
    static HISTORY: History = WINDOW.with(|w| w.history().unwrap_js());
}

pub(crate) fn body() -> HtmlElement {
    DOCUMENT.with(|d| d.body().unwrap_throw())
}

pub(crate) fn ready_state() -> String {
    DOCUMENT.with(|d| d.ready_state())
}

#[track_caller]
pub(crate) fn current_url() -> String {
    WINDOW.with(|w| w.location().href().unwrap_js())
}

#[track_caller]
pub(crate) fn go_to_url(url: &str) {
    HISTORY.with(|h| {
        h.push_state_with_url(&JsValue::NULL, "", Some(url)).unwrap_js();
    });
}

#[track_caller]
pub(crate) fn replace_url(url: &str) {
    HISTORY.with(|h| {
        h.replace_state_with_url(&JsValue::NULL, "", Some(url)).unwrap_js();
    });
}

#[track_caller]
pub(crate) fn create_stylesheet(css: Option<&str>) -> CssStyleSheet {
    DOCUMENT.with(|document| {
        // TODO use createElementNS ?
        // TODO use dyn_into ?
        let e: HtmlStyleElement = document.create_element("style").unwrap_js().unchecked_into();
        e.set_type("text/css");

        if let Some(css) = css {
            e.set_text_content(Some(css));
        }

        append_child(&document.head().unwrap_throw(), &e);
        // TODO use dyn_into ?
        e.sheet().unwrap_throw().unchecked_into()
    })
}

#[track_caller]
pub(crate) fn make_rule(sheet: &CssStyleSheet, rule: &str) -> Result<CssRule, JsValue> {
    let rules = sheet.css_rules().unwrap_js();
    let length = rules.length();
    // TODO don't return u32 ?
    sheet.insert_rule_with_index(rule, length)?;
    // TODO use dyn_into ?
    Ok(rules.get(length).unwrap_throw())
}


pub(crate) fn get_element_by_id(id: &str) -> Element {
    DOCUMENT.with(|d| d.get_element_by_id(id).unwrap_throw())
}

#[track_caller]
pub(crate) fn create_element(name: &str) -> Element {
    DOCUMENT.with(|d| d.create_element(name).unwrap_js())
}

#[track_caller]
pub(crate) fn create_element_ns(namespace: &str, name: &str) -> Element {
    DOCUMENT.with(|d| d.create_element_ns(Some(namespace), name).unwrap_js())
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
#[track_caller]
pub(crate) fn set_attribute(elem: &Element, key: &str, value: &str) {
    elem.set_attribute(key, value).unwrap_js();
}

#[track_caller]
pub(crate) fn set_attribute_ns(elem: &Element, namespace: &str, key: &str, value: &str) {
    elem.set_attribute_ns(Some(namespace), key, value).unwrap_js();
}

#[track_caller]
pub(crate) fn remove_attribute(elem: &Element, key: &str) {
    elem.remove_attribute(key).unwrap_js();
}

#[track_caller]
pub(crate) fn remove_attribute_ns(elem: &Element, namespace: &str, key: &str) {
    elem.remove_attribute_ns(Some(namespace), key).unwrap_js();
}

#[track_caller]
pub(crate) fn add_class(classes: &DomTokenList, value: &str) {
    classes.add_1(value).unwrap_js();
}

#[track_caller]
pub(crate) fn remove_class(classes: &DomTokenList, value: &str) {
    classes.remove_1(value).unwrap_js();
}

#[track_caller]
pub(crate) fn get_style(style: &CssStyleDeclaration, name: &str) -> String {
    style.get_property_value(name).unwrap_js()
}

#[track_caller]
pub(crate) fn remove_style(style: &CssStyleDeclaration, name: &str) {
    // TODO don't return String ?
    style.remove_property(name).unwrap_js();
}

#[track_caller]
pub(crate) fn set_style(style: &CssStyleDeclaration, name: &str, value: &str, important: bool) {
    let priority = if important { intern("important") } else { intern("") };
    style.set_property_with_priority(name, value, priority).unwrap_js();
}

#[track_caller]
pub(crate) fn append_raw(style: &CssStyleDeclaration, css: &str) {
    style.set_css_text(&(style.css_text() + css));
}

#[track_caller]
pub(crate) fn insert_child_before(parent: &Node, child: &Node, other: &Node) {
    // TODO don't return Node ?
    parent.insert_before(child, Some(other)).unwrap_js();
}

#[track_caller]
pub(crate) fn replace_child(parent: &Node, new: &Node, old: &Node) {
    parent.replace_child(new, old).unwrap_js();
}

#[track_caller]
pub(crate) fn append_child(parent: &Node, child: &Node) {
    parent.append_child(child).unwrap_js();
}

#[track_caller]
pub(crate) fn remove_child(parent: &Node, child: &Node) {
    parent.remove_child(child).unwrap_js();
}

#[track_caller]
pub(crate) fn focus(elem: &HtmlElement) {
    elem.focus().unwrap_js();
}

#[track_caller]
pub(crate) fn blur(elem: &HtmlElement) {
    elem.blur().unwrap_js();
}
