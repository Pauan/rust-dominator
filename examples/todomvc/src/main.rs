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
use stdweb::web::HtmlElement;
use stdweb::unstable::TryInto;
use stdweb::traits::*;

use futures_signals::signal::{SignalExt, Mutable};
use futures_signals::signal_vec::{SignalVecExt, MutableVec};
use dominator::{Dom, text};


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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


#[derive(Serialize, Deserialize)]
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

    todo_list: MutableVec<Rc<Todo>>,

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
        self.todo_list.lock_mut().retain(|x| x.id != todo.id);
    }

    fn update_filter(&self) {
        let hash = document().location().unwrap().hash().unwrap();

        self.filter.set_neq(match hash.as_str() {
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
        .children(children)
    })
}

#[inline]
fn link(href: &str, t: &str) -> Dom {
    html!("a", {
        .attribute("href", href)
        .text(t)
    })
}

fn filter_button(state: Rc<State>, kind: Filter) -> Dom {
    html!("a", {
        .class_signal("selected", state.filter.signal()
            .map(clone!(kind => move |filter| filter == kind)))

        .attribute("href", match kind {
            Filter::Active => "#/active",
            Filter::Completed => "#/completed",
            Filter::All => "#/",
        })

        .text(match kind {
            Filter::Active => "Active",
            Filter::Completed => "Completed",
            Filter::All => "All",
        })
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

                            .event(clone!(state => move |event: InputEvent| {
                                state.new_todo_title.set_neq(get_value(&event));
                            }))

                            .event(clone!(state => move |event: KeyDownEvent| {
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

                            .event(clone!(state => move |event: ChangeEvent| {
                                let checked = !get_checked(&event);

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
                                                let filter = state.filter.signal(),
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

                                                        .event(clone!(state, todo => move |event: ChangeEvent| {
                                                            todo.completed.set_neq(get_checked(&event));
                                                            state.serialize();
                                                        }))
                                                    }),

                                                    html!("label", {
                                                        .event(clone!(todo => move |_: DoubleClickEvent| {
                                                            todo.editing.set_neq(Some(todo.title.get_cloned()));
                                                        }))

                                                        .text_signal(todo.title.signal_cloned())
                                                    }),

                                                    html!("button", {
                                                        .class("destroy")
                                                        .event(clone!(state, todo => move |_: ClickEvent| {
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

                                                .event(clone!(todo => move |event: KeyDownEvent| {
                                                    let key = event.key();

                                                    if key == "Enter" {
                                                        let element: HtmlElement = event.target().unwrap().try_into().unwrap();
                                                        element.blur();

                                                    } else if key == "Escape" {
                                                        todo.editing.set_neq(None);
                                                    }
                                                }))

                                                .event(clone!(todo => move |event: InputEvent| {
                                                    todo.editing.set_neq(Some(get_value(&event)));
                                                }))

                                                // TODO global_event ?
                                                .event(clone!(state, todo => move |_: BlurEvent| {
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
                                simple("li", &mut [
                                    filter_button(state.clone(), Filter::All),
                                ]),
                                simple("li", &mut [
                                    filter_button(state.clone(), Filter::Active),
                                ]),
                                simple("li", &mut [
                                    filter_button(state.clone(), Filter::Completed),
                                ]),
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

                            .event(clone!(state => move |_: ClickEvent| {
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

    dominator::append_dom(&body,
        html!("footer", {
            .class("info")
            .children(&mut [
                html!("p", {
                    .text("Double-click to edit a todo")
                }),
                simple("p", &mut [
                    text("Created by "),
                    link("https://github.com/Pauan", "Pauan"),
                ]),
                simple("p", &mut [
                    text("Part of "),
                    link("http://todomvc.com", "TodoMVC"),
                ]),
            ])
        }),
    );
}
