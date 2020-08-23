#![deny(warnings, clippy::pedantic, clippy::all)]
#![warn(rust_2018_idioms)]

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
    NothingToInsert,
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FailedToAcquireStore => write!(fmt, "failed to acquire store"),
            Self::SecretNotFound => write!(fmt, "secret not found"),
            Self::NothingToInsert => write!(fmt, "nothing to insert"),
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
        use gotham::handler::IntoHandlerError;

        if data.is_empty() {
            return Err(Error::NothingToInsert
                .into_handler_error()
                .with_status(gotham::hyper::StatusCode::BAD_REQUEST));
        }

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
            Err(Error::FailedToAcquireStore.into_handler_error())
        }
    }

    fn get(&mut self, key: &str) -> Result<Vec<u8>, gotham::handler::HandlerError> {
        use gotham::handler::IntoHandlerError;
        self.secrets
            .lock()
            .map_err(|_| Error::FailedToAcquireStore.into_handler_error())
            .and_then(|mut map| {
                map.remove(key).ok_or_else(|| {
                    Error::SecretNotFound
                        .into_handler_error()
                        .with_status(gotham::hyper::StatusCode::NOT_FOUND)
                })
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
        use gotham::router::builder::{DefineSingleRoute, DrawRoutes};

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

#[cfg(test)]
mod tests {
    use super::router;
    use gotham::test::TestServer;

    #[test]
    fn post_secret() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .post("http://localhost", "foo", mime::TEXT_PLAIN)
            .perform()
            .unwrap();

        // assert_eq!(response.status(), gotham::hyper::StatusCode::CREATED);
        assert_eq!(response.status(), gotham::hyper::StatusCode::OK);

        let body = response.read_body().unwrap();
        assert_eq!(body.len(), 44);
    }

    #[test]
    fn get_secret() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .post("http://localhost", "foo", mime::TEXT_PLAIN)
            .perform()
            .unwrap();

        let key = response.read_body().unwrap();

        let response = test_server
            .client()
            .get(format!(
                "http://localhost/{}",
                key.into_iter().map(|c| c as char).collect::<String>()
            ))
            .perform()
            .unwrap();

        assert_eq!(response.status(), gotham::hyper::StatusCode::OK);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"foo");
    }

    #[test]
    fn cannot_choose_key_to_put() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .post("http://localhost/my_key", "foo", mime::TEXT_PLAIN)
            .perform()
            .unwrap();

        assert_eq!(
            response.status(),
            gotham::hyper::StatusCode::METHOD_NOT_ALLOWED
        );

        let body = response.read_body().unwrap();
        assert!(body.is_empty());
    }

    #[test]
    fn cannot_put_empty_values() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .post("http://localhost", "", mime::TEXT_PLAIN)
            .perform()
            .unwrap();

        assert_eq!(response.status(), gotham::hyper::StatusCode::BAD_REQUEST);

        let body = response.read_body().unwrap();
        assert!(body.is_empty());
    }

    #[test]
    fn only_get_if_exists() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/foo")
            .perform()
            .unwrap();

        assert_eq!(response.status(), gotham::hyper::StatusCode::NOT_FOUND);

        let body = response.read_body().unwrap();
        assert!(body.is_empty());
    }
}
