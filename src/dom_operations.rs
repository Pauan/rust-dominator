use std;
use stdweb::unstable::{TryFrom, TryInto};
use stdweb::{Value, Reference, JsSerialize};
use stdweb::web::{TextNode, INode, IHtmlElement, IElement};


#[inline]
pub fn create_element_ns<A: IElement>(name: &str, namespace: &str) -> A
    where <A as TryFrom<Value>>::Error: std::fmt::Debug {
    js!( return document.createElementNS(@{namespace}, @{name}); ).try_into().unwrap()
}

// TODO make this more efficient
#[inline]
pub fn move_from_to<A: INode>(parent: &A, old_index: u32, new_index: u32) {
    js! { @(no_return)
        var parent = @{parent.as_ref()};
        // TODO verify that it exists ?
        var child = parent.childNodes[@{old_index}];
        parent.removeChild(child);
        parent.insertBefore(child, parent.childNodes[@{new_index}]);
    }
}

// TODO make this more efficient
#[inline]
pub fn insert_at<A: INode, B: INode>(parent: &A, index: u32, child: &B) {
    js! { @(no_return)
        var parent = @{parent.as_ref()};
        parent.insertBefore(@{child.as_ref()}, parent.childNodes[@{index}]);
    }
}

// TODO make this more efficient
#[inline]
pub fn update_at<A: INode, B: INode>(parent: &A, index: u32, child: &B) {
    js! { @(no_return)
        var parent = @{parent.as_ref()};
        parent.replaceChild(@{child.as_ref()}, parent.childNodes[@{index}]);
    }
}

// TODO make this more efficient
#[inline]
pub fn remove_at<A: INode>(parent: &A, index: u32) {
    js! { @(no_return)
        var parent = @{parent.as_ref()};
        parent.removeChild(parent.childNodes[@{index}]);
    }
}

// TODO this should be in stdweb
#[inline]
pub fn set_text(element: &TextNode, value: &str) {
    js! { @(no_return)
        // http://jsperf.com/textnode-performance
        @{element}.data = @{value};
    }
}


#[inline]
fn get_style<A: AsRef<Reference>>(element: &A, name: &str) -> String {
    js!( return @{element.as_ref()}.style.getPropertyValue(@{name}); ).try_into().unwrap()
}

#[inline]
fn set_style_raw<A: AsRef<Reference>>(element: &A, name: &str, value: &str, important: bool) {
    js! { @(no_return)
        @{element.as_ref()}.style.setProperty(@{name}, @{value}, (@{important} ? "important" : ""));
    }
}

// TODO this should be in stdweb
// TODO maybe use cfg(debug_assertions) ?
pub fn try_set_style<A: AsRef<Reference>>(element: &A, name: &str, value: &str, important: bool) -> bool {
    assert!(value != "");

    remove_style(element, name);
    set_style_raw(element, name, value, important);

    get_style(element, name) != ""
}

// TODO this should be in stdweb
// TODO handle browser prefixes
#[inline]
pub fn remove_style<A: AsRef<Reference>>(element: &A, name: &str) {
    js! { @(no_return)
        @{element.as_ref()}.style.removeProperty(@{name});
    }
}

// TODO replace with element.focus() and element.blur()
// TODO make element.focus() and element.blur() inline
#[inline]
pub fn set_focused<A: IHtmlElement>(element: &A, focused: bool) {
    js! { @(no_return)
        var element = @{element.as_ref()};

        if (@{focused}) {
            element.focus();

        } else {
            element.blur();
        }
    }
}

#[inline]
pub fn add_class<A: IElement>(element: &A, name: &str) {
    js! { @(no_return)
        @{element.as_ref()}.classList.add(@{name});
    }
}

#[inline]
pub fn remove_class<A: IElement>(element: &A, name: &str) {
    js! { @(no_return)
        @{element.as_ref()}.classList.remove(@{name});
    }
}


// TODO check that the attribute *actually* was changed
#[inline]
pub fn set_attribute<A: IElement>(element: &A, name: &str, value: &str) {
    js! { @(no_return)
        @{element.as_ref()}.setAttribute(@{name}, @{value});
    }
}

// TODO check that the attribute *actually* was changed
#[inline]
pub fn set_attribute_ns<A: IElement>(element: &A, namespace: &str, name: &str, value: &str) {
    js! { @(no_return)
        @{element.as_ref()}.setAttributeNS(@{namespace}, @{name}, @{value});
    }
}


#[inline]
pub fn remove_attribute_ns<A: IElement>(element: &A, namespace: &str, name: &str) {
    js! { @(no_return)
        @{element.as_ref()}.removeAttributeNS(@{namespace}, @{name});
    }
}

#[inline]
pub fn remove_attribute<A: IElement>(element: &A, name: &str) {
    js! { @(no_return)
        @{element.as_ref()}.removeAttribute(@{name});
    }
}


// TODO check that the property *actually* was changed ?
// TODO better type checks ?
#[inline]
pub fn set_property<A: AsRef<Reference>, B: JsSerialize>(obj: &A, name: &str, value: B) {
    js! { @(no_return)
        @{obj.as_ref()}[@{name}] = @{value};
    }
}


// TODO make this work on Nodes, not just Elements
// TODO is this the most efficient way to remove all children ?
#[inline]
pub fn remove_all_children<A: INode>(element: &A) {
    js! { @(no_return)
        @{element.as_ref()}.innerHTML = "";
    }
}
