#[macro_use]
extern crate stdweb;
#[macro_use]
extern crate dominator;
#[macro_use]
extern crate futures_signals;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

use std::rc::Rc;
use std::cell::Cell;

// TODO replace most of these with dominator
use stdweb::web::{window, document};
use stdweb::web::event::{InputEvent, ClickEvent, HashChangeEvent, KeyDownEvent, ChangeEvent, DoubleClickEvent, BlurEvent};
use stdweb::web::html_element::InputElement;
use stdweb::unstable::TryInto;
use stdweb::traits::*;

use futures_signals::signal::Signal;
use futures_signals::signal_vec::SignalVec;
use futures_signals::signal::unsync::Mutable;
use futures_signals::signal_vec::unsync::MutableVec;
use dominator::traits::*;
use dominator::{Dom, text};


#[derive(Clone, Copy, PartialEq, Eq)]
enum Filter {
    Active,
    Completed,
    All,
}

impl Default for Filter {
    #[inline]
    fn default() -> Self {
        Filter::All
    }
}


#[derive(Clone, Serialize, Deserialize)]
struct Todo {
    id: u32,
    title: Mutable<String>,
    completed: Mutable<bool>,

    #[serde(skip)]
    editing: Mutable<Option<String>>,
}


#[derive(Serialize, Deserialize)]
struct State {
    todo_id: Cell<u32>,

    #[serde(skip)]
    new_todo_title: Mutable<String>,

    todo_list: MutableVec<Todo>,

    #[serde(skip)]
    filter: Mutable<Filter>,
}

impl State {
    fn new() -> Self {
        State {
            todo_id: Cell::new(0),
            new_todo_title: Mutable::new("".to_owned()),
            todo_list: MutableVec::new(),
            filter: Mutable::new(Filter::All),
        }
    }

    fn remove_todo(&self, todo: &Todo) {
        // TODO make this more efficient ?
        self.todo_list.retain(|x| x.id != todo.id);
    }

    fn update_filter(&self) {
        let hash = document().location().unwrap().hash().unwrap();

        self.filter.set(match hash.as_str() {
            "#/active" => Filter::Active,
            "#/completed" => Filter::Completed,
            _ => Filter::All,
        });
    }

    fn deserialize() -> Self {
        window().local_storage().get("todos-rust-dominator").and_then(|state_json| {
            serde_json::from_str(state_json.as_str()).ok()
        }).unwrap_or_else(State::new)
    }

    fn serialize(&self) {
        let state_json = serde_json::to_string(self).unwrap();
        window().local_storage().insert("todos-rust-dominator", state_json.as_str()).unwrap();
    }
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

#[inline]
fn get_value(event: &InputEvent) -> String {
    let target: InputElement = event.target().unwrap().try_into().unwrap();
    target.raw_value()
}

#[inline]
fn get_checked(event: &ChangeEvent) -> bool {
    js!( return @{&event.target()}.checked; ).try_into().unwrap()
}

#[inline]
fn simple(kind: &str, children: &mut [Dom]) -> Dom {
    html!(kind, {
        children(children);
    })
}

#[inline]
fn link(href: &str, t: &str) -> Dom {
    html!("a", {
        attribute("href", href);
        children(&mut [
            text(t),
        ]);
    })
}

fn filter_button(state: Rc<State>, kind: Filter) -> Dom {
    html!("a", {
        class("selected", state.filter.signal()
            .map(clone!(kind => move |filter| filter == kind))
            .dynamic());

        attribute("href", match kind {
            Filter::Active => "#/active",
            Filter::Completed => "#/completed",
            Filter::All => "#/",
        });

        children(&mut [
            text(match kind {
                Filter::Active => "Active",
                Filter::Completed => "Completed",
                Filter::All => "All",
            })
        ]);
    })
}


fn main() {
    let state = Rc::new(State::deserialize());

    state.update_filter();

    window().add_event_listener(clone!(state => move |_: HashChangeEvent| {
        state.update_filter();
    }));


    let body = dominator::body();

    dominator::append_dom(&body,
        html!("section", {
            class("todoapp", true);
            children(&mut [
                html!("header", {
                    class("header", true);
                    children(&mut [
                        simple("h1", &mut [
                            text("todos"),
                        ]),
                        html!("input", {
                            focused(true);
                            class("new-todo", true);
                            attribute("placeholder", "What needs to be done?");

                            property("value", state.new_todo_title.signal_cloned().dynamic());

                            event(clone!(state => move |event: InputEvent| {
                                state.new_todo_title.set(get_value(&event));
                            }));

                            event(clone!(state => move |event: KeyDownEvent| {
                                if event.key() == "Enter" {
                                    event.prevent_default();

                                    let trimmed = trim(&state.new_todo_title.borrow());

                                    if let Some(title) = trimmed {
                                        state.new_todo_title.set("".to_owned());

                                        let id = state.todo_id.get();

                                        state.todo_id.set(id + 1);

                                        state.todo_list.push_cloned(Todo {
                                            id: id,
                                            title: Mutable::new(title),
                                            completed: Mutable::new(false),
                                            editing: Mutable::new(None),
                                        });

                                        state.serialize();
                                    }
                                }
                            }));
                        }),
                    ]);
                }),

                html!("section", {
                    class("main", true);

                    // Hide if it doesn't have any todos.
                    property("hidden", state.todo_list.signal_vec_cloned()
                        .len()
                        .map(|len| len == 0)
                        .dynamic());

                    children(&mut [
                        html!("input", {
                            class("toggle-all", true);
                            attribute("id", "toggle-all");
                            attribute("type", "checkbox");

                            property("checked", state.todo_list.signal_vec_cloned()
                                .map_signal(|todo| todo.completed.signal())
                                .filter(|completed| !completed)
                                .len()
                                .map(|len| len != 0)
                                .dynamic());

                            event(clone!(state => move |event: ChangeEvent| {
                                let checked = !get_checked(&event);

                                for todo in state.todo_list.as_slice().iter() {
                                    todo.completed.set(checked);
                                }

                                state.serialize();
                            }));
                        }),

                        html!("label", {
                            attribute("for", "toggle-all");
                            children(&mut [
                                text("Mark all as complete"),
                            ]);
                        }),

                        html!("ul", {
                            class("todo-list", true);

                            children(state.todo_list.signal_vec_cloned()
                                .map(clone!(state => move |todo| {
                                    html!("li", {
                                        class("editing", todo.editing.signal_cloned()
                                            .map(|x| x.is_some())
                                            .dynamic());

                                        class("completed", todo.completed.signal().dynamic());

                                        property("hidden",
                                            map_cloned!(
                                                let filter = state.filter.signal(),
                                                let completed = todo.completed.signal() =>
                                                match filter {
                                                    Filter::Active => !completed,
                                                    Filter::Completed => completed,
                                                    Filter::All => true,
                                                }
                                            )
                                            .map_dedupe(|show| !*show)
                                            .dynamic());

                                        children(&mut [
                                            html!("div", {
                                                class("view", true);
                                                children(&mut [
                                                    html!("input", {
                                                        attribute("type", "checkbox");
                                                        class("toggle", true);

                                                        property("checked", todo.completed.signal().dynamic());

                                                        event(clone!(state, todo => move |event: ChangeEvent| {
                                                            todo.completed.set(get_checked(&event));
                                                            state.serialize();
                                                        }));
                                                    }),

                                                    html!("label", {
                                                        event(clone!(todo => move |_: DoubleClickEvent| {
                                                            todo.editing.set(Some(todo.title.get_cloned()));
                                                        }));

                                                        children(&mut [
                                                            text(todo.title.signal_cloned().dynamic()),
                                                        ]);
                                                    }),

                                                    html!("button", {
                                                        class("destroy", true);
                                                        event(clone!(state, todo => move |_: ClickEvent| {
                                                            state.remove_todo(&todo);
                                                            state.serialize();
                                                        }));
                                                    }),
                                                ]);
                                            }),

                                            html!("input", {
                                                class("edit", true);

                                                property("value", todo.editing.signal_cloned()
                                                    .map(|x| x.unwrap_or_else(|| "".to_owned()))
                                                    .dynamic());

                                                property("hidden", todo.editing.signal_cloned()
                                                    .map(|x| x.is_none())
                                                    .dynamic());

                                                // TODO dedupe this somehow ?
                                                focused(todo.editing.signal_cloned()
                                                    .map(|x| x.is_some())
                                                    .dynamic());

                                                event(clone!(todo => move |event: KeyDownEvent| {
                                                    let key = event.key();

                                                    if key == "Enter" {
                                                        let element: HtmlElement = event.target().unwrap().try_into().unwrap();
                                                        element.blur();

                                                    } else if key == "Escape" {
                                                        todo.editing.set(None);
                                                    }
                                                }));

                                                event(clone!(todo => move |event: InputEvent| {
                                                    todo.editing.set(Some(get_value(&event)));
                                                }));

                                                // TODO global_event ?
                                                event(clone!(state, todo => move |_: BlurEvent| {
                                                    if let Some(title) = todo.editing.replace(None) {
                                                        if let Some(title) = trim(&title) {
                                                            todo.title.set(title);

                                                        } else {
                                                            state.remove_todo(&todo);
                                                        }

                                                        state.serialize();
                                                    }
                                                }));
                                            }),
                                        ]);
                                    })
                                }))
                                .dynamic()
                            );
                        }),
                    ]);
                }),

                html!("footer", {
                    class("footer", true);

                    // Hide if it doesn't have any todos.
                    property("hidden", state.todo_list.signal_vec_cloned()
                        .len()
                        .map(|len| len == 0)
                        .dynamic());

                    children(&mut [
                        html!("span", {
                            class("todo-count", true);

                            children(state.todo_list.signal_vec_cloned()
                                .map_signal(|todo| todo.completed.signal())
                                .filter(|completed| !completed)
                                .len()
                                // TODO make this more efficient
                                .map(|len| {
                                    vec![
                                        simple("strong", &mut [
                                            text(len.to_string())
                                        ]),
                                        text(if len == 1 {
                                            " item left"
                                        } else {
                                            " items left"
                                        }),
                                    ]
                                })
                                .to_signal_vec()
                                .dynamic()
                            );
                        }),
                        html!("ul", {
                            class("filters", true);
                            children(&mut [
                                simple("li", &mut [
                                    filter_button(state.clone(), Filter::All),
                                ]),
                                simple("li", &mut [
                                    filter_button(state.clone(), Filter::Active),
                                ]),
                                simple("li", &mut [
                                    filter_button(state.clone(), Filter::Completed),
                                ]),
                            ]);
                        }),
                        html!("button", {
                            class("clear-completed", true);

                            // Hide if it doesn't have any completed items.
                            property("hidden", state.todo_list.signal_vec_cloned()
                                .map_signal(|todo| todo.completed.signal())
                                .filter(|completed| *completed)
                                .len()
                                .map(|len| len == 0)
                                .dynamic());

                            event(clone!(state => move |_: ClickEvent| {
                                state.todo_list.retain(|todo| todo.completed.get() == false);
                                state.serialize();
                            }));

                            children(&mut [
                                text("Clear completed"),
                            ]);
                        }),
                    ]);
                }),
            ]);
        }),
    );

    dominator::append_dom(&body,
        html!("footer", {
            class("info", true);
            children(&mut [
                simple("p", &mut [
                    text("Double-click to edit a todo"),
                ]),
                simple("p", &mut [
                    text("Created by "),
                    link("https://github.com/Pauan", "Pauan"),
                ]),
                simple("p", &mut [
                    text("Part of "),
                    link("http://todomvc.com", "TodoMVC"),
                ]),
            ]);
        }),
    );
}
