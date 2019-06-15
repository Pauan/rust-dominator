use wasm_bindgen::UnwrapThrowExt;
use web_sys::{Node, HtmlElement, Element};


#[inline]
pub(crate) fn get_at(parent: &Node, index: u32) -> Node {
    parent.child_nodes().get(index).unwrap_throw()
}

// TODO make this more efficient
#[inline]
pub(crate) fn move_from_to(parent: &Node, old_index: u32, new_index: u32) {
    let child = get_at(parent, old_index);

    parent.remove_child(&child).unwrap_throw();

    insert_at(parent, new_index, &child);
}

// TODO make this more efficient
#[inline]
pub(crate) fn insert_at(parent: &Node, index: u32, child: &Node) {
    parent.insert_before(child, Some(&get_at(parent, index))).unwrap_throw();
}

// TODO make this more efficient
#[inline]
pub(crate) fn update_at(parent: &Node, index: u32, child: &Node) {
    parent.replace_child(child, &get_at(parent, index)).unwrap_throw();
}

// TODO make this more efficient
#[inline]
pub(crate) fn remove_at(parent: &Node, index: u32) {
    parent.remove_child(&get_at(parent, index)).unwrap_throw();
}


#[inline]
pub(crate) fn set_focused(element: &HtmlElement, focused: bool) {
    if focused {
        element.focus().unwrap_throw();

    } else {
        element.blur().unwrap_throw();
    }
}

#[inline]
pub(crate) fn add_class(element: &Element, name: &str) {
    element.class_list().add_1(name).unwrap_throw();
}

#[inline]
pub(crate) fn remove_class(element: &Element, name: &str) {
    element.class_list().remove_1(name).unwrap_throw();
}


// TODO check that the attribute *actually* was changed
#[inline]
pub(crate) fn set_attribute(element: &Element, name: &str, value: &str) {
    element.set_attribute(name, value).unwrap_throw();
}

// TODO check that the attribute *actually* was changed
#[inline]
pub(crate) fn set_attribute_ns(element: &Element, namespace: &str, name: &str, value: &str) {
    element.set_attribute_ns(Some(namespace), name, value).unwrap_throw();
}


#[inline]
pub(crate) fn remove_attribute_ns(element: &Element, namespace: &str, name: &str) {
    element.remove_attribute_ns(Some(namespace), name).unwrap_throw();
}

#[inline]
pub(crate) fn remove_attribute(element: &Element, name: &str) {
    element.remove_attribute(name).unwrap_throw();
}


#[inline]
pub(crate) fn remove_all_children(node: &Node) {
    node.set_text_content(None);
}
