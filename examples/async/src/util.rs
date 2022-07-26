use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use futures_signals::signal::{Signal, Mutable};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use futures::future::{abortable, AbortHandle};
use js_sys::Error;
use web_sys::{window, Response, RequestInit, Headers, AbortController, AbortSignal};


struct AsyncState {
    id: usize,
    handle: AbortHandle,
}

impl AsyncState {
    fn new(handle: AbortHandle) -> Self {
        static ID: AtomicUsize = AtomicUsize::new(0);

        let id = ID.fetch_add(1, Ordering::SeqCst);

        Self { id, handle }
    }
}

pub struct AsyncLoader {
    loading: Mutable<Option<AsyncState>>,
}

impl AsyncLoader {
    pub fn new() -> Self {
        Self {
            loading: Mutable::new(None),
        }
    }

    pub fn cancel(&self) {
        self.replace(None);
    }

    fn replace(&self, value: Option<AsyncState>) {
        let mut loading = self.loading.lock_mut();

        if let Some(state) = loading.as_mut() {
            state.handle.abort();
        }

        *loading = value;
    }

    pub fn load<F>(&self, fut: F) where F: Future<Output = ()> + 'static {
        let (fut, handle) = abortable(fut);

        let state = AsyncState::new(handle);
        let id = state.id;

        self.replace(Some(state));

        let loading = self.loading.clone();

        spawn_local(async move {
            match fut.await {
                Ok(()) => {
                    let mut loading = loading.lock_mut();

                    if let Some(current_id) = loading.as_ref().map(|x| x.id) {
                        // If it hasn't been overwritten with a new state...
                        if current_id == id {
                            *loading = None;
                        }
                    }
                },
                // It was already cancelled
                Err(_) => {},
            }
        });
    }

    pub fn is_loading(&self) -> impl Signal<Item = bool> {
        self.loading.signal_ref(|x| x.is_some())
    }
}


struct Abort {
    controller: AbortController,
}

impl Abort {
    fn new() -> Result<Self, JsValue> {
        Ok(Self {
            controller: AbortController::new()?,
        })
    }

    fn signal(&self) -> AbortSignal {
        self.controller.signal()
    }
}

impl Drop for Abort {
    fn drop(&mut self) {
        self.controller.abort();
    }
}

pub async fn fetch_github(url: &str) -> Result<String, JsValue> {
    let abort = Abort::new()?;

    let headers = Headers::new()?;
    headers.set("Accept", "application/vnd.github.v3+json")?;

    let future = window()
        .unwrap()
        .fetch_with_str_and_init(
            url,
            RequestInit::new()
                .headers(&headers)
                .signal(Some(&abort.signal())),
        );

    let response = JsFuture::from(future)
        .await?
        .unchecked_into::<Response>();

    if !response.ok() {
        return Err(Error::new("Fetch failed").into());
    }

    let value = JsFuture::from(response.text()?)
        .await?
        .as_string()
        .unwrap();

    Ok(value)
}
