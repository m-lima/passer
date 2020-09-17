use super::store;
use gotham::hyper;

#[derive(Debug)]
enum Error {
    FailedToAcquireStore,
    SecretNotFound,
    NothingToInsert,
    TooLarge,
    StoreFull,
    InvalidExpiry,
    Unknown,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FailedToAcquireStore => write!(fmt, "failed to acquire store"),
            Self::SecretNotFound => write!(fmt, "secret not found"),
            Self::NothingToInsert => write!(fmt, "nothing to insert"),
            Self::TooLarge => write!(fmt, "payload too large"),
            Self::StoreFull => write!(fmt, "store is full"),
            Self::InvalidExpiry => write!(fmt, "invalid expiry time"),
            Self::Unknown => write!(fmt, "unknown error"),
        }
    }
}

impl Error {
    fn to_handler_error(self) -> gotham::handler::HandlerError {
        let status = match &self {
            Self::FailedToAcquireStore => hyper::StatusCode::INTERNAL_SERVER_ERROR,
            Self::SecretNotFound => hyper::StatusCode::NOT_FOUND,
            Self::NothingToInsert => hyper::StatusCode::BAD_REQUEST,
            Self::TooLarge => hyper::StatusCode::PAYLOAD_TOO_LARGE,
            Self::StoreFull => hyper::StatusCode::CONFLICT,
            Self::InvalidExpiry => hyper::StatusCode::BAD_REQUEST,
            Self::Unknown => hyper::StatusCode::INTERNAL_SERVER_ERROR,
        };
        gotham::handler::HandlerError::from(self).with_status(status)
    }
}

impl std::convert::From<store::Error> for Error {
    fn from(err: store::Error) -> Self {
        match err {
            store::Error::SecretNotFound => Self::SecretNotFound,
            store::Error::Unknown(_) => Self::Unknown,
        }
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
            // Allowed because this is third-party code being flagged
            #[allow(clippy::used_underscore_binding)]
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
                                    |addr| format!("{}", addr.ip().to_string()),
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
pub struct Store<S: store::Store + Send + 'static> {
    store: std::sync::Arc<std::sync::Mutex<S>>,
}

impl<S: store::Store + Send + 'static> Store<S> {
    pub fn new(store: S) -> Self {
        Self {
            store: std::sync::Arc::new(std::sync::Mutex::new(store)),
        }
    }

    pub fn put(
        &mut self,
        data: Vec<u8>,
        expiry: std::time::SystemTime,
    ) -> Result<String, gotham::handler::HandlerError> {
        if data.is_empty() {
            return Err(Error::NothingToInsert.to_handler_error());
        }

        let size = data.len() as u64;
        if size > store::MAX_SECRET_SIZE {
            return Err(Error::TooLarge.to_handler_error());
        }

        if expiry <= std::time::SystemTime::now() {
            return Err(Error::InvalidExpiry.to_handler_error());
        }

        let millis = expiry
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| Error::Unknown.to_handler_error())?
            .as_millis();

        let mut store = self
            .store
            .lock()
            .map_err(|_| Error::FailedToAcquireStore.to_handler_error())?;

        store.refresh();

        if store.size() + size > S::max_size() {
            return Err(Error::StoreFull.to_handler_error());
        }

        store
            .put(expiry, data)
            .map_err(|_| Error::Unknown.to_handler_error())
    }

    pub fn get(&mut self, key: &str) -> Result<Vec<u8>, gotham::handler::HandlerError> {
        let mut store = self
            .store
            .lock()
            .map_err(|_| Error::FailedToAcquireStore.to_handler_error())?;

        store.refresh();

        store
            .get(key)
            .map_err(|e| Error::from(e).to_handler_error())
    }
}
