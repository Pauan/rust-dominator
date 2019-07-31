use std::rc::Rc;

use wasm_bindgen::prelude::*;
use serde_derive::{Serialize, Deserialize};
use futures_signals::map_ref;
use futures_signals::signal::{SignalExt, Mutable};
use dominator::{Dom, html, clone, events, with_node};

use crate::util::trim;
use crate::app::App;
use crate::routing::Route;


#[derive(Debug, Serialize, Deserialize)]
pub struct Todo {
    id: u32,
    title: Mutable<String>,
    pub completed: Mutable<bool>,

    #[serde(skip)]
    editing: Mutable<Option<String>>,
}

impl Todo {
    pub fn new(id: u32, title: String) -> Rc<Self> {
        Rc::new(Self {
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

    fn cancel_editing(&self) {
        self.editing.set_neq(None);
    }

    fn done_editing(&self, app: &App) {
        if let Some(title) = self.editing.replace(None) {
            if let Some(title) = trim(&title) {
                self.title.set_neq(title);

            } else {
                app.remove_todo(&self);
            }

            app.serialize();
        }
    }

    pub fn render(todo: Rc<Self>, app: Rc<App>) -> Dom {
        html!("li", {
            .class_signal("editing", todo.editing.signal_cloned().map(|x| x.is_some()))
            .class_signal("completed", todo.completed.signal())

            .visible_signal(map_ref!(
                    let route = Route::signal(),
                    let completed = todo.completed.signal() =>
                    match *route {
                        Route::Active => !completed,
                        Route::Completed => *completed,
                        Route::All => true,
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

                            .event(clone!(todo, app => move |event: events::Change| {
                                todo.set_completed(&app, event.checked().unwrap_throw());
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
                            .event(clone!(todo, app => move |_: events::Click| {
                                todo.remove(&app);
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

                    .event(clone!(todo => move |event: events::Input| {
                        todo.editing.set_neq(Some(event.value().unwrap_throw()));
                    }))

                    // TODO global_event ?
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
