use gotham::hyper;

use super::middleware;

#[derive(Debug)]
enum Error {
    FailedToAcquireStore,
    SecretNotFound,
    NothingToInsert,
    TooLarge,
    StoreFull,
    InvalidExpiry,
    Unknown(String),
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
            Self::Unknown(msg) => write!(fmt, "unknown error: {}", msg),
        }
    }
}

impl std::convert::From<middleware::Error> for Error {
    fn from(e: middleware::Error) -> Self {
        use super::store::Error as StoreError;
        use middleware::Error;

        match e {
            Error::FailedToAcquireStore => Self::FailedToAcquireStore,
            Error::SecretNotFound => Self::SecretNotFound,
            Error::Store(StoreError::TooLarge) => Self::TooLarge,
            Error::Store(StoreError::StoreFull) => Self::StoreFull,
            Error::Store(StoreError::InvalidExpiry) => Self::InvalidExpiry,
            Error::Store(e) => Self::Unknown(e.to_string()),
        }
    }
}

fn map_to_handler_error(e: Error) -> gotham::handler::HandlerError {
    let status = match &e {
        Error::SecretNotFound => hyper::StatusCode::NOT_FOUND,
        Error::TooLarge => hyper::StatusCode::PAYLOAD_TOO_LARGE,
        Error::StoreFull => hyper::StatusCode::CONFLICT,
        Error::NothingToInsert | Error::InvalidExpiry => hyper::StatusCode::BAD_REQUEST,
        Error::FailedToAcquireStore | Error::Unknown(_) => hyper::StatusCode::INTERNAL_SERVER_ERROR,
    };

    log::warn!("{}", &e);
    gotham::handler::HandlerError::from(e).with_status(status)
}

fn map_to_response(e: &Error, state: &gotham::state::State) -> hyper::Response<hyper::Body> {
    let status = match &e {
        Error::SecretNotFound => hyper::StatusCode::NOT_FOUND,
        Error::TooLarge => hyper::StatusCode::PAYLOAD_TOO_LARGE,
        Error::StoreFull => hyper::StatusCode::CONFLICT,
        Error::NothingToInsert | Error::InvalidExpiry => hyper::StatusCode::BAD_REQUEST,
        Error::FailedToAcquireStore | Error::Unknown(_) => hyper::StatusCode::INTERNAL_SERVER_ERROR,
    };

    log::warn!("{}", &e);
    gotham::helpers::http::response::create_empty_response(state, status)
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

    let response = store.get(&id).map_or_else(
        |e| map_to_response(&e.into(), &state),
        |r| r.into_response(&state),
    );
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
                        &data,
                        std::time::SystemTime::now()
                            .checked_add(std::time::Duration::from_secs(24 * 60 * 60))
                            .unwrap(),
                    )
                    .map(|key| {
                        let mut response = key.into_response(&state);
                        *response.status_mut() = hyper::StatusCode::CREATED;
                        response
                    })
                    .map_err(|e| e.into())
            }) {
            Ok(r) => Ok((state, r)),
            Err(e) => Err((state, map_to_handler_error(e))),
        }
    })
}
