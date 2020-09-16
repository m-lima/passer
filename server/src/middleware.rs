use gotham::hyper;

use super::Error;

static MAX_LENGTH: usize = 110 * 1024 * 1024;
static MAX_STORE_SIZE: usize = 10;

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
                        .map(|fwd| format!("{}[p]", fwd))
                        .unwrap_or_else(|| {
                            gotham::state::client_addr(&state).map_or_else(
                                || String::from("??"),
                                |addr| format!("{}[r]", addr.ip().to_string()),
                            )
                        });

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

#[derive(Clone, Default, gotham_derive::StateData)]
pub struct Store {
    secrets: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>>,
}

impl Store {
    pub fn new() -> Self {
        Self::default()
    }

    fn new_key() -> String {
        use rand::Rng;
        base64::encode_config(
            rand::thread_rng().gen::<[u8; 32]>(),
            base64::URL_SAFE_NO_PAD,
        )
    }

    pub fn put(&mut self, data: Vec<u8>) -> Result<String, gotham::handler::HandlerError> {
        use gotham::handler::MapHandlerError;

        if data.is_empty() {
            return Err(Error::NothingToInsert).map_err_with_status(hyper::StatusCode::BAD_REQUEST);
        }

        if data.len() > MAX_LENGTH {
            return Err(Error::TooLarge).map_err_with_status(hyper::StatusCode::PAYLOAD_TOO_LARGE);
        }

        if let Ok(mut map) = self.secrets.lock() {
            if map.len() > MAX_STORE_SIZE {
                Err(Error::StoreFull).map_err_with_status(hyper::StatusCode::CONFLICT)
            } else {
                let key = loop {
                    let key = Self::new_key();
                    if !map.contains_key(&key) {
                        break key;
                    }
                };

                map.insert(key.clone(), data);
                Ok(key)
            }
        } else {
            Err(Error::FailedToAcquireStore)
                .map_err_with_status(hyper::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }

    pub fn get(&mut self, key: &str) -> Result<Vec<u8>, gotham::handler::HandlerError> {
        self.secrets
            .lock()
            .map_err(|_| Error::FailedToAcquireStore.into())
            .and_then(|mut map| {
                map.remove(key).ok_or_else(|| {
                    let err: gotham::handler::HandlerError = Error::SecretNotFound.into();
                    err.with_status(hyper::StatusCode::NOT_FOUND)
                })
            })
    }
}
