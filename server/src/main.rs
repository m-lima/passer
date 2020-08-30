#![deny(warnings, clippy::pedantic, clippy::all)]
#![warn(rust_2018_idioms)]

use gotham::hyper;

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

#[derive(serde::Deserialize, gotham_derive::StateData, gotham_derive::StaticResponseExtender)]
struct IdExtractor {
    id: String,
}

#[cfg(feature = "local-dev")]
#[derive(Clone, gotham_derive::NewMiddleware)]
struct CorsMiddleware;

#[cfg(feature = "local-dev")]
impl gotham::middleware::Middleware for CorsMiddleware {
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
                header.insert(
                    hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN,
                    hyper::header::HeaderValue::from_static("http://localhost:3000"),
                );
                (state, response)
            })
        })
    }
}

#[derive(Clone, gotham_derive::NewMiddleware)]
struct LogMiddleware;

impl gotham::middleware::Middleware for LogMiddleware {
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

                    let ip = gotham::state::client_addr(&state)
                        .map(|addr| addr.ip().to_string())
                        .unwrap_or_else(|| String::from("??"));

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
struct Store {
    secrets: std::sync::Arc<std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>>,
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
                .with_status(hyper::StatusCode::BAD_REQUEST));
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
                        .with_status(hyper::StatusCode::NOT_FOUND)
                })
            })
    }
}

fn get_handler(
    mut state: gotham::state::State,
) -> (gotham::state::State, hyper::Response<hyper::Body>) {
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
        use gotham::state::FromState;
        use hyper::{body, Body};

        // Allowed because this is third-party code being flagged
        #[allow(clippy::used_underscore_binding)]
        match body::to_bytes(Body::take_from(&mut state))
            .await
            .map(|b| b.to_vec())
            .map_err(IntoHandlerError::into_handler_error)
            .and_then(|data| {
                let store = Store::borrow_mut_from(&mut state);
                store.put(data).map(|key| {
                    let mut response = key.into_response(&state);
                    *response.status_mut() = hyper::StatusCode::CREATED;
                    response
                })
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

    let pipeline = pipeline::new_pipeline()
        .add(LogMiddleware)
        .add(StateMiddleware::new(Store::new()));

    #[cfg(feature = "local-dev")]
    let pipeline = pipeline.add(CorsMiddleware).build();
    #[cfg(not(feature = "local-dev"))]
    let pipeline = pipeline.build();

    let (chain, pipelines) = pipeline::single::single_pipeline(pipeline);

    let routes = builder::build_router(chain, pipelines, |route| {
        use gotham::router::builder::{DefineSingleRoute, DrawRoutes};

        #[cfg(feature = "local-dev")]
        route.options("/").to(|state| (state, ""));
        route.post("/").to(post_handler);
        route
            .get_or_head("/:id")
            .with_path_extractor::<IdExtractor>()
            .to(get_handler)
    });

    routes
}

fn init_logger() {
    let config = simplelog::ConfigBuilder::new()
        .set_time_format_str("%Y-%m-%dT%H:%M:%SZ")
        .build();

    simplelog::TermLogger::init(
        simplelog::LevelFilter::Info,
        config,
        simplelog::TerminalMode::Mixed,
    )
    .expect("Could not initialize logger");
}

fn main() {
    init_logger();

    let port = std::env::args()
        .nth(1)
        .map_or(Ok(80_u16), |port| port.parse::<u16>())
        .unwrap_or_else(|e| {
            log::error!("Inalid port: {}", e);
            std::process::exit(-1)
        });

    gotham::start_with_num_threads(format!("0.0.0.0:{}", port), router(), 1);
}

#[cfg(test)]
mod tests {
    use super::router;
    use gotham::hyper;
    use gotham::test::TestServer;

    #[test]
    fn post_secret() {
        let test_server = TestServer::new(router()).unwrap();
        let response = test_server
            .client()
            .post("http://localhost", "foo", mime::TEXT_PLAIN)
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::CREATED);

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

        assert_eq!(response.status(), hyper::StatusCode::OK);

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

        assert_eq!(response.status(), hyper::StatusCode::METHOD_NOT_ALLOWED);

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

        assert_eq!(response.status(), hyper::StatusCode::BAD_REQUEST);

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

        assert_eq!(response.status(), hyper::StatusCode::NOT_FOUND);

        let body = response.read_body().unwrap();
        assert!(body.is_empty());
    }
}
