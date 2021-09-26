use super::store;

use gotham::hyper;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to acquire store")]
    FailedToAcquireStore,
    #[error("{0}")]
    Store(store::Error),
}

impl From<store::Error> for Error {
    fn from(e: store::Error) -> Self {
        Self::Store(e)
    }
}

#[derive(Clone, gotham_derive::NewMiddleware)]
pub struct Cors(hyper::header::HeaderValue);

impl Cors {
    pub fn new(cors: hyper::header::HeaderValue) -> Self {
        Self(cors)
    }
}

impl gotham::middleware::Middleware for Cors {
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
            chain(state).await.map(|(state, mut response)| {
                let header = response.headers_mut();
                header.insert(hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN, self.0);
                (state, response)
            })
        })
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

#[derive(Clone, gotham_derive::StateData, gotham_derive::NewMiddleware)]
pub struct Store(std::sync::Arc<std::sync::Mutex<dyn 'static + store::Store + Send>>);

impl Store {
    pub fn new(store: impl 'static + store::Store + Send) -> Self {
        Self(std::sync::Arc::new(std::sync::Mutex::new(store)))
    }

    pub fn put(
        &mut self,
        data: Vec<u8>,
        expiry: std::time::SystemTime,
    ) -> Result<store::Id, Error> {
        let mut store = self.0.lock().map_err(|_| Error::FailedToAcquireStore)?;
        store.refresh();
        store.put(expiry, data).map_err(Error::Store)
    }

    pub fn get(&mut self, key: &store::Id) -> Result<Vec<u8>, Error> {
        let mut store = self.0.lock().map_err(|_| Error::FailedToAcquireStore)?;
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
