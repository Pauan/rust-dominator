use std::rc::Rc;
use std::cell::Cell;

use wasm_bindgen::prelude::*;
use serde_derive::{Serialize, Deserialize};
use futures_signals::signal::{Signal, SignalExt, Mutable};
use futures_signals::signal_vec::{SignalVec, SignalVecExt, MutableVec};
use dominator::{Dom, text_signal, html, clone, events, link};

use crate::todo::Todo;
use crate::routing::Route;
use crate::util::{trim, local_storage};


#[derive(Debug, Serialize, Deserialize)]
pub struct App {
    todo_id: Cell<u32>,

    #[serde(skip)]
    new_todo_title: Mutable<String>,

    todo_list: MutableVec<Rc<Todo>>,
}

impl App {
    fn new() -> Rc<Self> {
        Rc::new(App {
            todo_id: Cell::new(0),
            new_todo_title: Mutable::new("".to_owned()),
            todo_list: MutableVec::new(),
        })
    }

    pub fn deserialize() -> Rc<Self> {
        local_storage()
            .get_item("todos-rust-dominator")
            .unwrap_throw()
            .and_then(|state_json| {
                serde_json::from_str(state_json.as_str()).ok()
            })
            .unwrap_or_else(App::new)
    }

    pub fn serialize(&self) {
        let state_json = serde_json::to_string(self).unwrap_throw();

        local_storage()
            .set_item("todos-rust-dominator", state_json.as_str())
            .unwrap_throw();
    }

    fn create_new_todo(&self) {
        let trimmed = trim(&self.new_todo_title.lock_ref());

        // Only create a new Todo if the text box is not empty
        if let Some(title) = trimmed {
            self.new_todo_title.set_neq("".to_owned());

            let id = self.todo_id.get();
            self.todo_id.set(id + 1);
            self.todo_list.lock_mut().push_cloned(Todo::new(id, title));

            self.serialize();
        }
    }

    pub fn remove_todo(&self, todo: &Todo) {
        // TODO make this more efficient ?
        self.todo_list.lock_mut().retain(|x| **x != *todo);
    }

    fn remove_all_completed_todos(&self) {
        self.todo_list.lock_mut().retain(|todo| todo.completed.get() == false);
    }

    fn set_all_todos_completed(&self, checked: bool) {
        for todo in self.todo_list.lock_ref().iter() {
            todo.completed.set_neq(checked);
        }

        self.serialize();
    }

    fn completed(&self) -> impl SignalVec<Item = bool> {
        self.todo_list.signal_vec_cloned()
            .map_signal(|todo| todo.completed.signal())
    }

    fn completed_len(&self) -> impl Signal<Item = usize> {
        self.completed()
            .filter(|completed| *completed)
            .len()
    }

    fn not_completed_len(&self) -> impl Signal<Item = usize> {
        self.completed()
            .filter(|completed| !completed)
            .len()
    }

    fn render_header(app: Rc<Self>) -> Dom {
        html!("header", {
            .class("header")
            .children(vec![
                html!("h1", {
                    .text("todos")
                }),

                html!("input", {
                    .focused(true)
                    .class("new-todo")
                    .attribute("placeholder", "What needs to be done?")
                    .property_signal("value", app.new_todo_title.signal_cloned())

                    .event(clone!(app => move |event: events::Input| {
                        app.new_todo_title.set_neq(event.value().unwrap_throw());
                    }))

                    .event_preventable(clone!(app => move |event: events::KeyDown| {
                        if event.key() == "Enter" {
                            event.prevent_default();
                            Self::create_new_todo(&app);
                        }
                    }))
                }),
            ])
        })
    }

    fn render_main(app: Rc<Self>) -> Dom {
        html!("section", {
            .class("main")

            // Hide if it doesn't have any todos.
            .visible_signal(app.todo_list.signal_vec_cloned()
                .len()
                .map(|len| len > 0))

            .children(vec![
                html!("input", {
                    .class("toggle-all")
                    .attribute("id", "toggle-all")
                    .attribute("type", "checkbox")
                    .property_signal("checked", app.not_completed_len().map(|len| len == 0))

                    .event(clone!(app => move |event: events::Change| {
                        let checked = event.checked().unwrap_throw();
                        app.set_all_todos_completed(checked);
                    }))
                }),

                html!("label", {
                    .attribute("for", "toggle-all")
                    .text("Mark all as complete")
                }),

                html!("ul", {
                    .class("todo-list")
                    .children_signal_vec(app.todo_list.signal_vec_cloned()
                        .map(clone!(app => move |todo| Todo::render(todo, app.clone()))))
                }),
            ])
        })
    }

    fn render_button(text: &str, route: Route) -> Dom {
        html!("li", {
            .children(vec![
                link!(route.url(), {
                    .text(text)
                    .class_signal("selected", Route::signal().map(move |x| x == route))
                })
            ])
        })
    }

    fn render_footer(app: Rc<Self>) -> Dom {
        html!("footer", {
            .class("footer")

            // Hide if it doesn't have any todos.
            .visible_signal(app.todo_list.signal_vec_cloned()
                .len()
                .map(|len| len > 0))

            .children(vec![
                html!("span", {
                    .class("todo-count")

                    .children(vec![
                        html!("strong", {
                            .text_signal(app.not_completed_len().map(|len| len.to_string()))
                        }),

                        text_signal(app.not_completed_len().map(|len| {
                            if len == 1 {
                                " item left"
                            } else {
                                " items left"
                            }
                        })),
                    ])
                }),

                html!("ul", {
                    .class("filters")
                    .children(vec![
                        Self::render_button("All", Route::All),
                        Self::render_button("Active", Route::Active),
                        Self::render_button("Completed", Route::Completed),
                    ])
                }),

                html!("button", {
                    .class("clear-completed")

                    // Show if there is at least one completed item.
                    .visible_signal(app.completed_len().map(|len| len > 0))

                    .event(clone!(app => move |_: events::Click| {
                        app.remove_all_completed_todos();
                        app.serialize();
                    }))

                    .text("Clear completed")
                }),
            ])
        })
    }

    pub fn render(app: Rc<Self>) -> Dom {
        html!("section", {
            .class("todoapp")
            .children(vec![
                Self::render_header(app.clone()),
                Self::render_main(app.clone()),
                Self::render_footer(app.clone()),
            ])
        })
    }
}
