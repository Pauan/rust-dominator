use std;
use stdweb::unstable::{TryFrom, TryInto};
use stdweb::{Value, Reference};
use stdweb::web::{TextNode, INode, IHtmlElement, IElement};


#[inline]
pub fn create_element_ns<A: IElement>(name: &str, namespace: &str) -> A
    where <A as TryFrom<Value>>::Error: std::fmt::Debug {
    js!( return document.createElementNS(@{namespace}, @{name}); ).try_into().unwrap()
}

#[inline]
pub fn insert_at<A: INode, B: INode>(parent: &A, index: u32, child: &B) {
    js! { @(no_return)
        var parent = @{parent.as_ref()};
        parent.insertBefore(@{child.as_ref()}, parent.childNodes[@{index}]);
    }
}

#[inline]
pub fn update_at<A: INode, B: INode>(parent: &A, index: u32, child: &B) {
    js! { @(no_return)
        var parent = @{parent.as_ref()};
        parent.replaceChild(@{child.as_ref()}, parent.childNodes[@{index}]);
    }
}

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
pub fn toggle_class<A: IElement>(element: &A, name: &str, toggle: bool) {
    js! { @(no_return)
        @{element.as_ref()}.classList.toggle(@{name}, @{toggle});
    }
}


#[inline]
fn _set_attribute_ns<A: IElement>(element: &A, name: &str, value: &str, namespace: &str) {
    js! { @(no_return)
        @{element.as_ref()}.setAttributeNS(@{namespace}, @{name}, @{value});
    }
}

#[inline]
fn _set_attribute<A: IElement>(element: &A, name: &str, value: &str) {
    js! { @(no_return)
        @{element.as_ref()}.setAttribute(@{name}, @{value});
    }
}

// TODO check that the attribute *actually* was changed
#[inline]
pub fn set_attribute<A: IElement>(element: &A, name: &str, value: &str, namespace: Option<&str>) {
    match namespace {
        Some(namespace) => _set_attribute_ns(element, name, value, namespace),
        None => _set_attribute(element, name, value),
    }
}


#[inline]
fn _remove_attribute_ns<A: IElement>(element: &A, name: &str, namespace: &str) {
    js! { @(no_return)
        @{element.as_ref()}.removeAttributeNS(@{namespace}, @{name});
    }
}

#[inline]
fn _remove_attribute<A: IElement>(element: &A, name: &str) {
    js! { @(no_return)
        @{element.as_ref()}.removeAttribute(@{name});
    }
}

#[inline]
pub fn remove_attribute<A: IElement>(element: &A, name: &str, namespace: Option<&str>) {
    match namespace {
        Some(namespace) => _remove_attribute_ns(element, name, namespace),
        None => _remove_attribute(element, name),
    }
}


// TODO check that the property *actually* was changed ?
#[inline]
pub fn set_property<A: AsRef<Reference>>(obj: &A, name: &str, value: Option<&str>) {
    // TODO make this more efficient
    match value {
        Some(value) => js! { @(no_return)
            @{obj.as_ref()}[@{name}] = @{value};
        },

        None => js! { @(no_return)
            @{obj.as_ref()}[@{name}] = null;
        },
    };
}


// TODO make this work on Nodes, not just Elements
// TODO is this the most efficient way to remove all children ?
#[inline]
pub fn remove_all_children<A: INode>(element: &A) {
    js! { @(no_return)
        @{element.as_ref()}.innerHTML = "";
    }
}
