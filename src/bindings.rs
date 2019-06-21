use wasm_bindgen::prelude::*;
use js_sys::{Function, JsString};
use web_sys::{HtmlElement, Element, Node, Window, Text, Comment, CssStyleSheet, CssStyleRule, EventTarget};

use crate::cache::intern;


#[wasm_bindgen(inline_js = "
    export function body() { return document.body; }
    export function _window() { return window; }

    export function ready_state() { return document.readyState; }

    export function current_url() { return location.href; }
    export function go_to_url(url) { history.pushState(null, \"\", url); }

    export function create_stylesheet() {
        // TODO use createElementNS ?
        var e = document.createElement(\"style\");
        e.type = \"text/css\";
        document.head.appendChild(e);
        return e.sheet;
    }

    export function make_style_rule(sheet, selector) {
        var rules = sheet.cssRules;
        var length = rules.length;
        sheet.insertRule(selector + \" {}\", length);
        return rules[length];
    }

    export function create_element(name) { return document.createElement(name); }
    export function create_element_ns(namespace, name) { return document.createElementNS(namespace, name); }

    export function create_text_node(value) { return document.createTextNode(value); }

    // http://jsperf.com/textnode-performance
    export function set_text(elem, value) { elem.data = value; }

    export function create_comment(value) { return document.createComment(value); }

    export function set_attribute(elem, key, value) { elem.setAttribute(key, value); }
    export function set_attribute_ns(elem, namespace, key, value) { elem.setAttributeNS(namespace, key, value); }

    export function remove_attribute(elem, key) { elem.removeAttribute(key); }
    export function remove_attribute_ns(elem, namespace, key) { elem.removeAttributeNS(namespace, key); }

    export function add_class(elem, value) { elem.classList.add(value); }
    export function remove_class(elem, value) { elem.classList.remove(value); }

    export function set_text_content(elem, value) { elem.textContent = value; }

    export function get_style(elem, name) { return elem.style.getPropertyValue(name); }
    export function remove_style(elem, name) { return elem.style.removeProperty(name); }

    export function set_style(elem, name, value, important) {
        elem.style.setProperty(name, value, (important ? \"important\" : \"\"));
    }

    export function get_at(parent, index) { return parent.childNodes[index]; }
    export function insert_child_before(parent, child, other) { parent.insertBefore(child, other); }
    export function replace_child(parent, child, other) { parent.replaceChild(child, other); }
    export function append_child(parent, child) { parent.appendChild(child); }
    export function remove_child(parent, child) { parent.removeChild(child); }

    export function focus(elem) { elem.focus(); }
    export function blur(elem) { elem.blur(); }

    export function set_property(obj, name, value) { obj[name] = value; }

    export function add_event(elem, name, f) {
        elem.addEventListener(name, f, {
            capture: false,
            once: false,
            passive: true
        });
    }

    export function add_event_once(elem, name, f) {
        elem.addEventListener(name, f, {
            capture: false,
            once: true,
            passive: true,
        });
    }

    export function add_event_preventable(elem, name, f) {
        elem.addEventListener(name, f, {
            capture: false,
            once: false,
            passive: false
        });
    }

    export function remove_event(elem, name, f) {
        elem.removeEventListener(name, f, false);
    }
")]
extern "C" {
    pub(crate) fn body() -> HtmlElement;

    #[wasm_bindgen(js_name = _window)]
    pub(crate) fn window() -> Window;

    pub(crate) fn ready_state() -> JsString;

    pub(crate) fn current_url() -> JsString;
    pub(crate) fn go_to_url(url: &JsString);

    pub(crate) fn create_stylesheet() -> CssStyleSheet;
    pub(crate) fn make_style_rule(sheet: &CssStyleSheet, selector: &JsString) -> CssStyleRule;

    pub(crate) fn create_element(name: &JsString) -> Element;
    pub(crate) fn create_element_ns(namespace: &JsString, name: &JsString) -> Element;

    pub(crate) fn create_text_node(value: &JsString) -> Text;
    pub(crate) fn set_text(elem: &Text, value: &JsString);

    pub(crate) fn create_comment(value: &JsString) -> Comment;

    // TODO check that the attribute *actually* was changed
    pub(crate) fn set_attribute(elem: &Element, key: &JsString, value: &JsString);
    pub(crate) fn set_attribute_ns(elem: &Element, namespace: &JsString, key: &JsString, value: &JsString);

    pub(crate) fn remove_attribute(elem: &Element, key: &JsString);
    pub(crate) fn remove_attribute_ns(elem: &Element, namespace: &JsString, key: &JsString);

    pub(crate) fn add_class(elem: &Element, value: &JsString);
    pub(crate) fn remove_class(elem: &Element, value: &JsString);

    pub(crate) fn set_text_content(elem: &Node, value: &JsString);

    // TODO better type for elem
    pub(crate) fn get_style(elem: &JsValue, name: &JsString) -> JsString;
    pub(crate) fn remove_style(elem: &JsValue, name: &JsString);
    pub(crate) fn set_style(elem: &JsValue, name: &JsString, value: &JsString, important: bool);

    pub(crate) fn get_at(parent: &Node, index: u32) -> Node;
    pub(crate) fn insert_child_before(parent: &Node, child: &Node, other: &Node);
    pub(crate) fn replace_child(parent: &Node, child: &Node, other: &Node);
    pub(crate) fn append_child(parent: &Node, child: &Node);
    pub(crate) fn remove_child(parent: &Node, child: &Node);

    pub(crate) fn focus(elem: &HtmlElement);
    pub(crate) fn blur(elem: &HtmlElement);

    // TODO maybe use Object for obj ?
    pub(crate) fn set_property(obj: &JsValue, name: &JsString, value: &JsValue);

    pub(crate) fn add_event(elem: &EventTarget, name: &JsString, f: &Function);
    pub(crate) fn add_event_once(elem: &EventTarget, name: &JsString, f: &Function);
    pub(crate) fn add_event_preventable(elem: &EventTarget, name: &JsString, f: &Function);
    pub(crate) fn remove_event(elem: &EventTarget, name: &JsString, f: &Function);
}


#[inline]
pub(crate) fn remove_all_children(node: &Node) {
    set_text_content(node, &intern(""));
}

// TODO make this more efficient
#[inline]
pub(crate) fn insert_at(parent: &Node, index: u32, child: &Node) {
    insert_child_before(parent, child, &get_at(parent, index));
}

// TODO make this more efficient
#[inline]
pub(crate) fn update_at(parent: &Node, index: u32, child: &Node) {
    replace_child(parent, child, &get_at(parent, index));
}

// TODO make this more efficient
#[inline]
pub(crate) fn remove_at(parent: &Node, index: u32) {
    remove_child(parent, &get_at(parent, index));
}
