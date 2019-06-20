use wasm_bindgen::{JsValue, UnwrapThrowExt};
use web_sys::{Node, HtmlElement, Element, DomTokenList};


#[inline]
pub(crate) fn get_at(parent: &Node, index: u32) -> Node {
    parent.child_nodes().get(index).unwrap_throw()
}

// TODO make this more efficient
#[inline]
pub(crate) fn move_from_to(parent: &Node, old_index: u32, new_index: u32) -> Result<(), JsValue> {
    let child = get_at(parent, old_index);

    parent.remove_child(&child)?;

    insert_at(parent, new_index, &child)?;

    Ok(())
}

// TODO make this more efficient
#[inline]
pub(crate) fn insert_at(parent: &Node, index: u32, child: &Node) -> Result<(), JsValue> {
    parent.insert_before(child, Some(&get_at(parent, index)))?;
    Ok(())
}

// TODO make this more efficient
#[inline]
pub(crate) fn update_at(parent: &Node, index: u32, child: &Node) -> Result<(), JsValue> {
    parent.replace_child(child, &get_at(parent, index))?;
    Ok(())
}

// TODO make this more efficient
#[inline]
pub(crate) fn remove_at(parent: &Node, index: u32) -> Result<(), JsValue> {
    parent.remove_child(&get_at(parent, index))?;
    Ok(())
}


#[inline]
pub(crate) fn set_focused(element: &HtmlElement, focused: bool) -> Result<(), JsValue> {
    if focused {
        element.focus()?;

    } else {
        element.blur()?;
    }

    Ok(())
}

#[inline]
pub(crate) fn add_class(list: &DomTokenList, name: &str) -> Result<(), JsValue> {
    list.add_1(name)?;
    Ok(())
}

#[inline]
pub(crate) fn remove_class(list: &DomTokenList, name: &str) -> Result<(), JsValue> {
    list.remove_1(name)?;
    Ok(())
}


// TODO check that the attribute *actually* was changed
#[inline]
pub(crate) fn set_attribute(element: &Element, name: &str, value: &str) -> Result<(), JsValue> {
    element.set_attribute(name, value)?;
    Ok(())
}

// TODO check that the attribute *actually* was changed
#[inline]
pub(crate) fn set_attribute_ns(element: &Element, namespace: &str, name: &str, value: &str) -> Result<(), JsValue> {
    element.set_attribute_ns(Some(namespace), name, value)?;
    Ok(())
}


#[inline]
pub(crate) fn remove_attribute_ns(element: &Element, namespace: &str, name: &str) -> Result<(), JsValue> {
    element.remove_attribute_ns(Some(namespace), name)?;
    Ok(())
}

#[inline]
pub(crate) fn remove_attribute(element: &Element, name: &str) -> Result<(), JsValue> {
    element.remove_attribute(name)?;
    Ok(())
}


#[inline]
pub(crate) fn remove_all_children(node: &Node) {
    node.set_text_content(None);
}
