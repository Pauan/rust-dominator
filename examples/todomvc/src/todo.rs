use std::sync::Arc;
use wasm_bindgen::prelude::*;
use serde_derive::{Serialize, Deserialize};
use futures_signals::map_ref;
use futures_signals::signal::{Signal, SignalExt, Mutable};
use dominator::{Dom, html, clone, events, with_node};
use web_sys::HtmlInputElement;

use crate::util::trim;
use crate::app::{App, Route};


#[derive(Debug, Serialize, Deserialize)]
pub struct Todo {
    id: u32,
    title: Mutable<String>,
    pub completed: Mutable<bool>,

    #[serde(skip)]
    editing: Mutable<Option<String>>,
}

impl Todo {
    pub fn new(id: u32, title: String) -> Arc<Self> {
        Arc::new(Self {
            id: id,
            title: Mutable::new(title),
            completed: Mutable::new(false),
            editing: Mutable::new(None),
        })
    }

    fn set_completed(&self, app: &App, completed: bool) {
        self.completed.set_neq(completed);
        app.serialize();
    }

    fn remove(&self, app: &App) {
        app.remove_todo(&self);
        app.serialize();
    }

    fn is_visible(&self, app: &App) -> impl Signal<Item = bool> {
        (map_ref! {
            let route = app.route(),
            let completed = self.completed.signal() =>
            match *route {
                Route::Active => !completed,
                Route::Completed => *completed,
                Route::All => true,
            }
        }).dedupe()
    }

    fn is_editing(&self) -> impl Signal<Item = bool> {
        self.editing.signal_ref(|x| x.is_some()).dedupe()
    }

    fn cancel_editing(&self) {
        self.editing.set_neq(None);
    }

    fn done_editing(&self, app: &App) {
        if let Some(title) = self.editing.replace(None) {
            if let Some(title) = trim(&title) {
                self.title.set_neq(title.to_string());

            } else {
                app.remove_todo(&self);
            }

            app.serialize();
        }
    }

    pub fn render(todo: Arc<Self>, app: Arc<App>) -> Dom {
        html!("li", {
            .class_signal("editing", todo.is_editing())
            .class_signal("completed", todo.completed.signal())

            .visible_signal(todo.is_visible(&app))

            .children(&mut [
                html!("div", {
                    .class("view")
                    .children(&mut [
                        html!("input" => HtmlInputElement, {
                            .class("toggle")
                            .attr("type", "checkbox")
                            .prop_signal("checked", todo.completed.signal())

                            .with_node!(element => {
                                .event(clone!(todo, app => move |_: events::Change| {
                                    todo.set_completed(&app, element.checked());
                                }))
                            })
                        }),

                        html!("label", {
                            .event(clone!(todo => move |_: events::DoubleClick| {
                                todo.editing.set_neq(Some(todo.title.get_cloned()));
                            }))

                            .text_signal(todo.title.signal_cloned())
                        }),

                        html!("button", {
                            .class("destroy")
                            .event(clone!(todo, app => move |_: events::Click| {
                                todo.remove(&app);
                            }))
                        }),
                    ])
                }),

                html!("input" => HtmlInputElement, {
                    .class("edit")

                    .prop_signal("value", todo.editing.signal_cloned()
                        .map(|x| x.unwrap_or_else(|| "".to_owned())))

                    .visible_signal(todo.is_editing())
                    .focused_signal(todo.is_editing())

                    .with_node!(element => {
                        .event(clone!(todo => move |event: events::KeyDown| {
                            match event.key().as_str() {
                                "Enter" => {
                                    element.blur().unwrap_throw();
                                },
                                "Escape" => {
                                    todo.cancel_editing();
                                },
                                _ => {}
                            }
                        }))
                    })

                    .with_node!(element => {
                        .event(clone!(todo => move |_: events::Input| {
                            todo.editing.set_neq(Some(element.value()));
                        }))
                    })

                    .event(clone!(todo, app => move |_: events::Blur| {
                        todo.done_editing(&app);
                    }))
                }),
            ])
        })
    }
}

impl PartialEq<Todo> for Todo {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
