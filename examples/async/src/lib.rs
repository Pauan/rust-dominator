use std::sync::Arc;
use wasm_bindgen::prelude::*;
use gloo_timers::future::TimeoutFuture;
use serde_derive::{Serialize, Deserialize};
use futures_signals::signal::{Mutable, not};
use dominator::{html, class, events, clone, with_node, Dom};
use web_sys::HtmlInputElement;
use once_cell::sync::Lazy;
use util::*;

mod util;


#[derive(Debug, Serialize, Deserialize)]
struct User {
    login: String,
    id: u32,
    node_id: String,
    avatar_url: String,
    gravatar_id: String,
    url: String,
    html_url: String,
    followers_url: String,
    following_url: String,
    gists_url: String,
    starred_url: String,
    subscriptions_url: String,
    repos_url: String,
    events_url: String,
    received_events_url: String,
    #[serde(rename = "type")]
    type_: String,
    site_admin: bool,
    name: Option<String>,
    company: Option<String>,
    blog: String,
    location: Option<String>,
    email: Option<String>,
    hireable: Option<bool>,
    bio: Option<String>,
    public_repos: u32,
    public_gists: u32,
    followers: u32,
    following: u32,
    created_at: String,
    updated_at: String,
}

impl User {
    async fn fetch(user: &str) -> Result<Self, JsValue> {
        let user = fetch_github(&format!("https://api.github.com/users/{}", user)).await?;
        Ok(serde_json::from_str::<Self>(&user).unwrap())
    }
}


struct App {
    user: Mutable<Option<User>>,
    input: Mutable<String>,
    loader: AsyncLoader,
}

impl App {
    fn new(name: &str, user: Option<User>) -> Arc<Self> {
        Arc::new(Self {
            user: Mutable::new(user),
            input: Mutable::new(name.to_string()),
            loader: AsyncLoader::new(),
        })
    }

    fn render(app: Arc<Self>) -> Dom {
        static APP: Lazy<String> = Lazy::new(|| class! {
            .style("white-space", "pre")
        });

        html!("div", {
            .class(&*APP)

            .children(&mut [
                html!("input" => HtmlInputElement, {
                    .prop_signal("value", app.input.signal_cloned())

                    .with_node!(element => {
                        .event(clone!(app => move |_: events::Input| {
                            app.input.set(element.value());
                        }))
                    })
                }),

                html!("button", {
                    .text("Lookup user")

                    .event(clone!(app => move |_: events::Click| {
                        let input = app.input.lock_ref();

                        if *input == "" {
                            app.user.set(None);

                        } else {
                            let input = input.to_string();

                            app.loader.load(clone!(app => async move {
                                // Simulate a slow network
                                TimeoutFuture::new(5_000).await;

                                let user = User::fetch(&input).await.ok();
                                app.user.set(user);
                            }));
                        }
                    }))
                }),

                html!("button", {
                    .text("Cancel")

                    .event(clone!(app => move |_: events::Click| {
                        app.loader.cancel();
                    }))
                }),

                html!("div", {
                    .visible_signal(app.loader.is_loading())
                    .text("LOADING")
                }),

                html!("div", {
                    .visible_signal(not(app.loader.is_loading()))

                    .text_signal(app.user.signal_ref(|user| format!("{:#?}", user)))
                }),
            ])
        })
    }
}


#[wasm_bindgen(start)]
pub async fn main_js() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let user = User::fetch("Pauan").await.ok();

    let app = App::new("Pauan", user);

    dominator::append_dom(&dominator::body(), App::render(app));

    Ok(())
}
