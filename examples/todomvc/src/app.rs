use std::sync::Arc;
use std::cell::Cell;
use wasm_bindgen::prelude::*;
use web_sys::{Url, HtmlInputElement};
use serde_derive::{Serialize, Deserialize};
use futures_signals::signal::{Signal, SignalExt, Mutable};
use futures_signals::signal_vec::{SignalVec, SignalVecExt, MutableVec};
use dominator::{Dom, EventOptions, text_signal, html, clone, events, link, with_node, routing};

use crate::todo::Todo;
use crate::util::{trim, local_storage};


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Active,
    Completed,
    All,
}

impl Route {
    // This could use more advanced URL parsing, but it isn't needed
    pub fn from_url(url: &str) -> Self {
        let url = Url::new(&url).unwrap_throw();
        match url.hash().as_str() {
            "#/active" => Route::Active,
            "#/completed" => Route::Completed,
            _ => Route::All,
        }
    }

    pub fn to_url(&self) -> &'static str {
        match self {
            Route::Active => "#/active",
            Route::Completed => "#/completed",
            Route::All => "#/",
        }
    }
}

impl Default for Route {
    fn default() -> Self {
        // Create the Route based on the current URL
        Self::from_url(&routing::url().lock_ref())
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub struct App {
    todo_id: Cell<u32>,

    #[serde(skip)]
    new_todo_title: Mutable<String>,

    todo_list: MutableVec<Arc<Todo>>,

    #[serde(skip)]
    route: Mutable<Route>,
}

impl App {
    fn new() -> Arc<Self> {
        Arc::new(App {
            todo_id: Cell::new(0),
            new_todo_title: Mutable::new("".to_owned()),
            todo_list: MutableVec::new(),
            route: Mutable::new(Route::default()),
        })
    }

    pub fn deserialize() -> Arc<Self> {
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

    pub fn route(&self) -> impl Signal<Item = Route> {
        self.route.signal()
    }

    fn create_new_todo(&self) {
        let mut title = self.new_todo_title.lock_mut();

        // Only create a new Todo if the text box is not empty
        if let Some(trimmed) = trim(&title) {
            let id = self.todo_id.get();
            self.todo_id.set(id + 1);

            self.todo_list.lock_mut().push_cloned(Todo::new(id, trimmed.to_string()));

            *title = "".to_string();

            self.serialize();
        }
    }

    pub fn remove_todo(&self, todo: &Todo) {
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

    fn has_todos(&self) -> impl Signal<Item = bool> {
        self.todo_list.signal_vec_cloned()
            .len()
            .map(|len| len > 0)
            .dedupe()
    }

    fn render_header(app: Arc<Self>) -> Dom {
        html!("header", {
            .class("header")
            .children(&mut [
                html!("h1", {
                    .text("todos")
                }),

                html!("input" => HtmlInputElement, {
                    .focused(true)
                    .class("new-todo")
                    .attr("placeholder", "What needs to be done?")
                    .prop_signal("value", app.new_todo_title.signal_cloned())

                    .with_node!(element => {
                        .event(clone!(app => move |_: events::Input| {
                            app.new_todo_title.set_neq(element.value());
                        }))
                    })

                    .event_with_options(&EventOptions::preventable(), clone!(app => move |event: events::KeyDown| {
                        if event.key() == "Enter" {
                            event.prevent_default();
                            app.create_new_todo();
                        }
                    }))
                }),
            ])
        })
    }

    fn render_main(app: Arc<Self>) -> Dom {
        html!("section", {
            .class("main")

            .visible_signal(app.has_todos())

            .children(&mut [
                html!("input" => HtmlInputElement, {
                    .class("toggle-all")
                    .attr("id", "toggle-all")
                    .attr("type", "checkbox")
                    .prop_signal("checked", app.not_completed_len().map(|len| len == 0).dedupe())

                    .with_node!(element => {
                        .event(clone!(app => move |_: events::Change| {
                            app.set_all_todos_completed(element.checked());
                        }))
                    })
                }),

                html!("label", {
                    .attr("for", "toggle-all")
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

    fn render_button(app: &App, text: &str, route: Route) -> Dom {
        html!("li", {
            .children(&mut [
                link!(route.to_url(), {
                    .text(text)
                    .class_signal("selected", app.route().map(move |x| x == route))
                })
            ])
        })
    }

    fn render_footer(app: Arc<Self>) -> Dom {
        html!("footer", {
            .class("footer")

            .visible_signal(app.has_todos())

            .children(&mut [
                html!("span", {
                    .class("todo-count")

                    .children(&mut [
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
                    .children(&mut [
                        Self::render_button(&app, "All", Route::All),
                        Self::render_button(&app, "Active", Route::Active),
                        Self::render_button(&app, "Completed", Route::Completed),
                    ])
                }),

                html!("button", {
                    .class("clear-completed")

                    // Show if there is at least one completed item.
                    .visible_signal(app.completed_len().map(|len| len > 0).dedupe())

                    .event(clone!(app => move |_: events::Click| {
                        app.remove_all_completed_todos();
                        app.serialize();
                    }))

                    .text("Clear completed")
                }),
            ])
        })
    }

    pub fn render(app: Arc<Self>) -> Dom {
        html!("section", {
            .class("todoapp")

            // Update the Route when the URL changes
            .future(routing::url()
                .signal_ref(|url| Route::from_url(url))
                .for_each(clone!(app => move |route| {
                    app.route.set_neq(route);
                    async {}
                })))

            .children(&mut [
                Self::render_header(app.clone()),
                Self::render_main(app.clone()),
                Self::render_footer(app.clone()),
            ])
        })
    }
}
