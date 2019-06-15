use std::rc::Rc;
use std::cell::Cell;

use wasm_bindgen::prelude::*;
use serde_derive::{Serialize, Deserialize};
use web_sys::{window, HtmlElement, Storage};
use futures_signals::map_ref;
use futures_signals::signal::{Signal, SignalExt, Mutable};
use futures_signals::signal_vec::{SignalVecExt, MutableVec};
use dominator::{Dom, text, routing, html, clone, events};


fn local_storage() -> Storage {
    window().unwrap_throw().local_storage().unwrap_throw().unwrap_throw()
}

// TODO make this more efficient
#[inline]
fn trim(input: &str) -> Option<String> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        None

    } else {
        Some(trimmed.to_owned())
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Filter {
    Active,
    Completed,
    All,
}

impl Filter {
    fn signal() -> impl Signal<Item = Self> {
        routing::url().map(|url| {
            match url.hash().as_str() {
                "#/active" => Filter::Active,
                "#/completed" => Filter::Completed,
                _ => Filter::All,
            }
        })
    }

    #[inline]
    fn button(kind: Self) -> Dom {
        let url = match kind {
            Filter::Active => "#/active",
            Filter::Completed => "#/completed",
            Filter::All => "#/",
        };

        let text = match kind {
            Filter::Active => "Active",
            Filter::Completed => "Completed",
            Filter::All => "All",
        };

        routing::link(url, |dom| { dom
            .class_signal("selected", Self::signal()
                .map(clone!(kind => move |filter| filter == kind)))
            .text(text)
        })
    }
}


#[derive(Debug, Serialize, Deserialize)]
struct Todo {
    id: u32,
    title: Mutable<String>,
    completed: Mutable<bool>,

    #[serde(skip)]
    editing: Mutable<Option<String>>,
}


#[derive(Debug, Serialize, Deserialize)]
struct State {
    todo_id: Cell<u32>,

    #[serde(skip)]
    new_todo_title: Mutable<String>,

    todo_list: MutableVec<Rc<Todo>>,
}

impl State {
    fn new() -> Self {
        State {
            todo_id: Cell::new(0),
            new_todo_title: Mutable::new("".to_owned()),
            todo_list: MutableVec::new(),
        }
    }

    fn remove_todo(&self, todo: &Todo) {
        // TODO make this more efficient ?
        self.todo_list.lock_mut().retain(|x| x.id != todo.id);
    }

    fn deserialize() -> Self {
        local_storage()
            .get_item("todos-rust-dominator")
            .unwrap_throw()
            .and_then(|state_json| {
                serde_json::from_str(state_json.as_str()).ok()
            })
            .unwrap_or_else(State::new)
    }

    fn serialize(&self) {
        let state_json = serde_json::to_string(self).unwrap_throw();

        local_storage().set_item("todos-rust-dominator", state_json.as_str()).unwrap_throw();
    }
}


#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    #[cfg(debug_assertions)]
    console_error_panic_hook::set_once();


    let state = Rc::new(State::deserialize());

    dominator::append_dom(&dominator::body(),
        html!("section", {
            .class("todoapp")
            .children(&mut [
                html!("header", {
                    .class("header")
                    .children(&mut [
                        html!("h1", {
                            .text("todos")
                        }),
                        html!("input", {
                            .focused(true)
                            .class("new-todo")
                            .attribute("placeholder", "What needs to be done?")

                            .property_signal("value", state.new_todo_title.signal_cloned())

                            .event(clone!(state => move |event: events::Input| {
                                state.new_todo_title.set_neq(event.value().unwrap_throw());
                            }))

                            .event(clone!(state => move |event: events::KeyDown| {
                                if event.key() == "Enter" {
                                    event.prevent_default();

                                    let trimmed = trim(&state.new_todo_title.lock_ref());

                                    if let Some(title) = trimmed {
                                        state.new_todo_title.set_neq("".to_owned());

                                        let id = state.todo_id.get();

                                        state.todo_id.set(id + 1);

                                        state.todo_list.lock_mut().push_cloned(Rc::new(Todo {
                                            id: id,
                                            title: Mutable::new(title),
                                            completed: Mutable::new(false),
                                            editing: Mutable::new(None),
                                        }));

                                        state.serialize();
                                    }
                                }
                            }))
                        }),
                    ])
                }),

                html!("section", {
                    .class("main")

                    // Hide if it doesn't have any todos.
                    .visible_signal(state.todo_list.signal_vec_cloned()
                        .len()
                        .map(|len| len > 0))

                    .children(&mut [
                        html!("input", {
                            .class("toggle-all")
                            .attribute("id", "toggle-all")
                            .attribute("type", "checkbox")

                            .property_signal("checked", state.todo_list.signal_vec_cloned()
                                .map_signal(|todo| todo.completed.signal())
                                .filter(|completed| !completed)
                                .len()
                                .map(|len| len != 0))

                            .event(clone!(state => move |event: events::Change| {
                                // Toggles the boolean
                                let checked = !event.checked().unwrap_throw();

                                {
                                    let todo_list = state.todo_list.lock_ref();

                                    for todo in todo_list.iter() {
                                        todo.completed.set_neq(checked);
                                    }
                                }

                                state.serialize();
                            }))
                        }),

                        html!("label", {
                            .attribute("for", "toggle-all")
                            .text("Mark all as complete")
                        }),

                        html!("ul", {
                            .class("todo-list")

                            .children_signal_vec(state.todo_list.signal_vec_cloned()
                                .map(clone!(state => move |todo| {
                                    html!("li", {
                                        .class_signal("editing", todo.editing.signal_cloned()
                                            .map(|x| x.is_some()))

                                        .class_signal("completed", todo.completed.signal())

                                        .visible_signal(map_ref!(
                                                let filter = Filter::signal(),
                                                let completed = todo.completed.signal() =>
                                                match *filter {
                                                    Filter::Active => !completed,
                                                    Filter::Completed => *completed,
                                                    Filter::All => true,
                                                }
                                            )
                                            .dedupe())

                                        .children(&mut [
                                            html!("div", {
                                                .class("view")
                                                .children(&mut [
                                                    html!("input", {
                                                        .attribute("type", "checkbox")
                                                        .class("toggle")

                                                        .property_signal("checked", todo.completed.signal())

                                                        .event(clone!(state, todo => move |event: events::Change| {
                                                            todo.completed.set_neq(event.checked().unwrap_throw());
                                                            state.serialize();
                                                        }))
                                                    }),

                                                    html!("label", {
                                                        .event(clone!(todo => move |_: events::DoubleClick| {
                                                            todo.editing.set_neq(Some(todo.title.get_cloned()));
                                                        }))

                                                        .text_signal(todo.title.signal_cloned())
                                                    }),

                                                    html!("button", {
                                                        .class("destroy")
                                                        .event(clone!(state, todo => move |_: events::Click| {
                                                            state.remove_todo(&todo);
                                                            state.serialize();
                                                        }))
                                                    }),
                                                ])
                                            }),

                                            html!("input", {
                                                .class("edit")

                                                .property_signal("value", todo.editing.signal_cloned()
                                                    .map(|x| x.unwrap_or_else(|| "".to_owned())))

                                                .visible_signal(todo.editing.signal_cloned()
                                                    .map(|x| x.is_some()))

                                                // TODO dedupe this somehow ?
                                                .focused_signal(todo.editing.signal_cloned()
                                                    .map(|x| x.is_some()))

                                                .event(clone!(todo => move |event: events::KeyDown| {
                                                    match event.key().as_str() {
                                                        "Enter" => {
                                                            event.dyn_target::<HtmlElement>()
                                                                .unwrap_throw()
                                                                .blur()
                                                                .unwrap_throw();
                                                        },
                                                        "Escape" => {
                                                            todo.editing.set_neq(None);
                                                        },
                                                        _ => {}
                                                    }
                                                }))

                                                .event(clone!(todo => move |event: events::Input| {
                                                    todo.editing.set_neq(Some(event.value().unwrap_throw()));
                                                }))

                                                // TODO global_event ?
                                                .event(clone!(state, todo => move |_: events::Blur| {
                                                    if let Some(title) = todo.editing.replace(None) {
                                                        if let Some(title) = trim(&title) {
                                                            todo.title.set_neq(title);

                                                        } else {
                                                            state.remove_todo(&todo);
                                                        }

                                                        state.serialize();
                                                    }
                                                }))
                                            }),
                                        ])
                                    })
                                })))
                        }),
                    ])
                }),

                html!("footer", {
                    .class("footer")

                    // Hide if it doesn't have any todos.
                    .visible_signal(state.todo_list.signal_vec_cloned()
                        .len()
                        .map(|len| len > 0))

                    .children(&mut [
                        html!("span", {
                            .class("todo-count")

                            .children_signal_vec(state.todo_list.signal_vec_cloned()
                                .map_signal(|todo| todo.completed.signal())
                                .filter(|completed| !completed)
                                .len()
                                // TODO make this more efficient
                                .map(|len| {
                                    vec![
                                        html!("strong", {
                                            .text(&len.to_string())
                                        }),
                                        text(if len == 1 {
                                            " item left"
                                        } else {
                                            " items left"
                                        }),
                                    ]
                                })
                                .to_signal_vec())
                        }),
                        html!("ul", {
                            .class("filters")
                            .children(&mut [
                                html!("li", {
                                    .children(&mut [
                                        Filter::button(Filter::All),
                                    ])
                                }),
                                html!("li", {
                                    .children(&mut [
                                        Filter::button(Filter::Active),
                                    ])
                                }),
                                html!("li", {
                                    .children(&mut [
                                        Filter::button(Filter::Completed),
                                    ])
                                }),
                            ])
                        }),
                        html!("button", {
                            .class("clear-completed")

                            // Hide if it doesn't have any completed items.
                            .visible_signal(state.todo_list.signal_vec_cloned()
                                .map_signal(|todo| todo.completed.signal())
                                .filter(|completed| *completed)
                                .len()
                                .map(|len| len > 0))

                            .event(clone!(state => move |_: events::Click| {
                                state.todo_list.lock_mut().retain(|todo| todo.completed.get() == false);
                                state.serialize();
                            }))

                            .text("Clear completed")
                        }),
                    ])
                }),
            ])
        }),
    );

    dominator::append_dom(&dominator::body(),
        html!("footer", {
            .class("info")
            .children(&mut [
                html!("p", {
                    .text("Double-click to edit a todo")
                }),
                html!("p", {
                    .children(&mut [
                        text("Created by "),
                        html!("a", {
                            .attribute("href", "https://github.com/Pauan")
                            .text("Pauan")
                        }),
                    ])
                }),
                html!("p", {
                    .children(&mut [
                        text("Part of "),
                        html!("a", {
                            .attribute("href", "http://todomvc.com")
                            .text("TodoMVC")
                        }),
                    ])
                }),
            ])
        }),
    );

    Ok(())
}
