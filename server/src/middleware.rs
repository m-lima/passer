use super::store;

use gotham::hyper;

#[derive(Debug)]
pub enum Error {
    FailedToAcquireStore,
    Store(store::Error),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FailedToAcquireStore => write!(fmt, "failed to acquire store"),
            Self::Store(e) => write!(fmt, "backend store error: {}", e),
        }
    }
}

impl std::convert::From<store::Error> for Error {
    fn from(e: store::Error) -> Self {
        Self::Store(e)
    }
}

#[derive(Clone, gotham_derive::NewMiddleware)]
pub struct Log;

impl gotham::middleware::Middleware for Log {
    fn call<C>(
        self,
        state: gotham::state::State,
        chain: C,
    ) -> std::pin::Pin<Box<gotham::handler::HandlerFuture>>
    where
        C: FnOnce(gotham::state::State) -> std::pin::Pin<Box<gotham::handler::HandlerFuture>>
            + Send
            + 'static,
    {
        Box::pin(async {
            // Allowed because this is third-party code being flagged
            #[allow(clippy::used_underscore_binding)]
            chain(state).await.map(|(state, response)| {
                {
                    use gotham::state::FromState;

                    let ip = hyper::HeaderMap::borrow_from(&state)
                        .get(hyper::header::HeaderName::from_static("x-forwarded-for"))
                        .and_then(|fwd| fwd.to_str().ok())
                        .map_or_else(
                            || {
                                gotham::state::client_addr(&state).map_or_else(
                                    || String::from("??"),
                                    |addr| addr.ip().to_string(),
                                )
                            },
                            |fwd| format!("{} [p]", fwd),
                        );

                    // Request info
                    let path = hyper::Uri::borrow_from(&state);
                    let method = hyper::Method::borrow_from(&state);
                    let length = hyper::HeaderMap::borrow_from(&state)
                        .get(hyper::header::CONTENT_LENGTH)
                        .and_then(|len| len.to_str().ok())
                        .unwrap_or("");

                    // Response info
                    let status = response.status().as_u16();

                    // Log out
                    log::info!("{} {} - {} {} {}", status, ip, method, path, length);
                }

                (state, response)
            })
        })
    }
}

#[derive(Clone, gotham_derive::StateData)]
pub struct Store {
    store: std::sync::Arc<std::sync::Mutex<dyn 'static + store::Store + Send>>,
}

impl Store {
    pub fn new(store: impl 'static + store::Store + Send) -> Self {
        Self {
            store: std::sync::Arc::new(std::sync::Mutex::new(store)),
        }
    }

    pub fn put(&mut self, data: Vec<u8>, expiry: std::time::SystemTime) -> Result<String, Error> {
        let mut store = self.store.lock().map_err(|_| Error::FailedToAcquireStore)?;
        store.refresh();
        store.put(expiry, data).map_err(Error::Store)
    }

    pub fn get(&mut self, key: &str) -> Result<Vec<u8>, Error> {
        let mut store = self.store.lock().map_err(|_| Error::FailedToAcquireStore)?;
        store.refresh();
        store.get(key).map_err(Error::Store)
    }
}

impl gotham::middleware::Middleware for Store {
    fn call<Chain>(
        self,
        mut state: gotham::state::State,
        chain: Chain,
    ) -> std::pin::Pin<Box<gotham::handler::HandlerFuture>>
    where
        Chain: FnOnce(gotham::state::State) -> std::pin::Pin<Box<gotham::handler::HandlerFuture>>,
    {
        state.put(self);
        chain(state)
    }
}

impl gotham::middleware::NewMiddleware for Store {
    type Instance = Self;

    fn new_middleware(&self) -> gotham::anyhow::Result<Self::Instance> {
        Ok(self.clone())
    }
}
