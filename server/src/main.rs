#[derive(serde::Deserialize, gotham_derive::StateData, gotham_derive::StaticResponseExtender)]
struct IdExtractor {
    id: String,
}

#[derive(Clone, Default, gotham_derive::StateData)]
struct Store {
    secrets: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>>,
}

#[derive(Debug)]
enum Error {
    FailedToAcquireStore,
    SecretNotFound,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FailedToAcquireStore => write!(fmt, "Failed to acquire store"),
            Self::SecretNotFound => write!(fmt, "Secret not found"),
        }
    }
}

impl Store {
    fn new() -> Self {
        Self::default()
    }

    fn new_key() -> String {
        use rand::Rng;
        base64::encode_config(rand::thread_rng().gen::<[u8; 32]>(), base64::URL_SAFE)
    }

    fn put(&mut self, data: Vec<u8>) -> Result<String, gotham::handler::HandlerError> {
        if let Ok(mut map) = self.secrets.lock() {
            let key = loop {
                let key = Self::new_key();
                if !map.contains_key(&key) {
                    break key;
                }
            };

            map.insert(key.clone(), data);
            Ok(key)
        } else {
            use gotham::handler::IntoHandlerError;
            Err(Error::FailedToAcquireStore.into_handler_error())
        }
    }

    fn get(&mut self, key: &str) -> Result<Vec<u8>, gotham::handler::HandlerError> {
        use gotham::handler::IntoHandlerError;
        self.secrets
            .lock()
            .map_err(|_| Error::FailedToAcquireStore.into_handler_error())
            .and_then(|mut map| {
                map.remove(key).ok_or(
                    Error::SecretNotFound
                        .into_handler_error()
                        .with_status(gotham::hyper::StatusCode::NOT_FOUND),
                )
            })
    }
}

fn get_handler(
    mut state: gotham::state::State,
) -> (
    gotham::state::State,
    gotham::hyper::Response<gotham::hyper::Body>,
) {
    use gotham::handler::IntoResponse;
    use gotham::state::FromState;

    let id = { IdExtractor::take_from(&mut state).id };
    let store = Store::borrow_mut_from(&mut state);

    let response = store
        .get(&id)
        .map_or_else(|e| e.into_response(&state), |r| r.into_response(&state));
    (state, response)
}

fn post_handler(
    mut state: gotham::state::State,
) -> std::pin::Pin<Box<gotham::handler::HandlerFuture>> {
    Box::pin(async {
        use gotham::handler::{IntoHandlerError, IntoResponse};
        use gotham::hyper::{body, Body};
        use gotham::state::FromState;

        match body::to_bytes(Body::take_from(&mut state))
            .await
            .map(|b| b.to_vec())
            .map_err(IntoHandlerError::into_handler_error)
            .and_then(|data| {
                let store = Store::borrow_mut_from(&mut state);
                store.put(data).map(|key| key.into_response(&state))
            }) {
            Ok(r) => Ok((state, r)),
            Err(e) => Err((state, e)),
        }
    })
}

fn router() -> gotham::router::Router {
    use gotham::middleware::state::StateMiddleware;
    use gotham::pipeline;
    use gotham::router::builder;

    let store_middleware = StateMiddleware::new(Store::new());
    let store_pipeline = pipeline::single_middleware(store_middleware);
    let (chain, pipelines) = pipeline::single::single_pipeline(store_pipeline);

    builder::build_router(chain, pipelines, |route| {
        use gotham::router::builder::*;

        route
            .get_or_head("/:id")
            .with_path_extractor::<IdExtractor>()
            .to(get_handler);
        route.post("/").to(post_handler)
    })
}

fn main() {
    gotham::start_with_num_threads("0.0.0.0:3030", router(), 1);
}
