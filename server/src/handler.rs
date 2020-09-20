use gotham::hyper;

use super::middleware;

#[derive(Debug)]
enum Error {
    NothingToInsert,
    InvalidExpiry,
    Middleware(middleware::Error),
    Unknown(String),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NothingToInsert => write!(fmt, "nothing to insert"),
            Self::InvalidExpiry => write!(fmt, "invalid expiry"),
            Self::Middleware(e) => write!(fmt, "{}", e),
            Self::Unknown(msg) => write!(fmt, "unknown error: {}", msg),
        }
    }
}

impl std::convert::From<middleware::Error> for Error {
    fn from(e: middleware::Error) -> Self {
        Self::Middleware(e)
    }
}

impl Error {
    fn status_code(&self) -> hyper::StatusCode {
        use super::store::Error as Store;
        use middleware::Error as Middleware;
        match self {
            Error::NothingToInsert | Error::InvalidExpiry => hyper::StatusCode::BAD_REQUEST,
            Error::Middleware(Middleware::Store(Store::TooLarge)) => {
                hyper::StatusCode::PAYLOAD_TOO_LARGE
            }
            Error::Middleware(Middleware::Store(Store::StoreFull)) => hyper::StatusCode::CONFLICT,
            Error::Middleware(Middleware::Store(Store::SecretNotFound)) => {
                hyper::StatusCode::NOT_FOUND
            }
            _ => hyper::StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn into_response(self, state: &gotham::state::State) -> hyper::Response<hyper::Body> {
        let status = self.status_code();
        log::warn!("{} [{}]", &self, &status);
        gotham::helpers::http::response::create_empty_response(state, status)
    }

    fn into_handler_error(self) -> gotham::handler::HandlerError {
        let status = self.status_code();
        log::warn!("{} [{}]", &self, &status);
        gotham::handler::HandlerError::from(self).with_status(status)
    }
}

#[derive(serde::Deserialize, gotham_derive::StateData, gotham_derive::StaticResponseExtender)]
pub struct IdExtractor {
    id: String,
}

// #[derive(gotham_derive::StateData, gotham_derive::StaticResponseExtender)]
// pub struct TtlExtractor {
//     ttl: std::time::Duration,
// }

// impl<'de> serde::Deserialize<'de> for TtlExtractor {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: serde::Deserializer<'de>,
//     {
//         struct AmountVisitor;

//         impl<'de> serde::de::Visitor<'de> for AmountVisitor {
//             type Value = u8;

//             fn expecting(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//                 fmt.write_str("a single digit integer greater than zero")
//             }

//             fn visit(
//         }
//     }
// }

#[cfg(feature = "host-frontend")]
#[derive(Clone)]
pub struct Index(
    gotham::handler::assets::DirHandler,
    gotham::handler::assets::FileHandler,
);

#[cfg(feature = "host-frontend")]
impl Index {
    pub fn new(root: std::path::PathBuf, index: std::path::PathBuf) -> Self {
        use gotham::handler::assets;
        Self(
            assets::DirHandler::new(assets::FileOptions::from(root)),
            assets::FileHandler::new(assets::FileOptions::from(index)),
        )
    }
}

#[cfg(feature = "host-frontend")]
impl gotham::handler::NewHandler for Index {
    type Instance = Self;

    fn new_handler(&self) -> gotham::anyhow::Result<Self::Instance> {
        Ok(self.clone())
    }
}

#[cfg(feature = "host-frontend")]
impl gotham::handler::Handler for Index {
    fn handle(
        self,
        state: gotham::state::State,
    ) -> std::pin::Pin<Box<gotham::handler::HandlerFuture>> {
        Box::pin(async {
            // Allowed because this is third-party code being flagged
            #[allow(clippy::used_underscore_binding)]
            match self.0.handle(state).await {
                Ok(response) => Ok(response),
                Err((state, _)) => self.1.handle(state).await,
            }
        })
    }
}

pub fn get(
    mut state: gotham::state::State,
) -> (gotham::state::State, hyper::Response<hyper::Body>) {
    use gotham::handler::IntoResponse;
    use gotham::state::FromState;

    let id = { IdExtractor::take_from(&mut state).id };
    let store = middleware::Store::borrow_mut_from(&mut state);

    let response = store
        .get(&id)
        .map_err(Error::from)
        .map_or_else(|e| e.into_response(&state), |r| r.into_response(&state));
    (state, response)
}

pub fn post(mut state: gotham::state::State) -> std::pin::Pin<Box<gotham::handler::HandlerFuture>> {
    Box::pin(async {
        use gotham::handler::IntoResponse;
        use gotham::state::FromState;
        use hyper::{body, Body};

        // Allowed because this is third-party code being flagged
        #[allow(clippy::used_underscore_binding)]
        match body::to_bytes(Body::take_from(&mut state))
            .await
            .map_err(|e| Error::Unknown(e.to_string()))
            .and_then(|bytes| {
                if bytes.is_empty() {
                    Err(Error::NothingToInsert)
                } else {
                    Ok(bytes.to_vec())
                }
            })
            .and_then(|data| {
                let store = middleware::Store::borrow_mut_from(&mut state);
                store
                    .put(
                        data,
                        std::time::SystemTime::now()
                            .checked_add(std::time::Duration::from_secs(24 * 60 * 60))
                            .unwrap(),
                    )
                    .map(|key| {
                        let mut response = key.into_response(&state);
                        *response.status_mut() = hyper::StatusCode::CREATED;
                        response
                    })
                    .map_err(Error::from)
            })
            .map_err(Error::into_handler_error)
        {
            Ok(r) => Ok((state, r)),
            Err(e) => Err((state, e)),
        }
    })
}
