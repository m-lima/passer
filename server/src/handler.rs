use gotham::hyper;

use super::middleware;
use super::store::Id;

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
        log::warn!("{}", &self);
        gotham::helpers::http::response::create_empty_response(state, status)
    }

    fn into_handler_error(self) -> gotham::handler::HandlerError {
        let status = self.status_code();
        log::warn!("{}", &self);
        gotham::handler::HandlerError::from(self).with_status(status)
    }
}

#[derive(serde::Deserialize, gotham_derive::StateData, gotham_derive::StaticResponseExtender)]
pub struct IdExtractor {
    #[serde(deserialize_with = "id_deserializer")]
    id: Id,
}

fn id_deserializer<'de, D>(deserializer: D) -> Result<Id, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct IdVisitor;

    impl<'de> serde::de::Visitor<'de> for IdVisitor {
        type Value = Id;

        fn expecting(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            fmt.write_str("a 32-bit unsigned integer base64 encoded")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Id::decode(value).map_err(|e| serde::de::Error::custom(e.to_string()))
        }
    }

    deserializer.deserialize_str(IdVisitor)
}

#[derive(serde::Deserialize, gotham_derive::StateData, gotham_derive::StaticResponseExtender)]
pub struct TtlExtractor {
    #[serde(deserialize_with = "duration_deserializer")]
    ttl: std::time::Duration,
}

fn duration_deserializer<'de, D>(deserializer: D) -> Result<std::time::Duration, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct TtlVisitor;

    impl<'de> serde::de::Visitor<'de> for TtlVisitor {
        type Value = std::time::Duration;

        fn expecting(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            fmt.write_str("a duration in <amount>[m|h|d] format")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            convert_str_to_duration(value).map_err(serde::de::Error::custom)
        }
    }

    deserializer.deserialize_str(TtlVisitor)
}

fn convert_str_to_duration(value: &str) -> Result<std::time::Duration, String> {
    let unit = match value.chars().last() {
        Some('m') => 60,
        Some('h') => 60 * 60,
        Some('d') => 24 * 60 * 60,
        Some(unit) => {
            return Err(format!("invalid duration unit: {}", unit));
        }
        None => {
            return Err(String::from("ttl is empty"));
        }
    };

    let amount = value[..value.len() - 1]
        .parse::<u64>()
        .map_err(|e| format!("could not parse amount: {}", e))?;
    Ok(std::time::Duration::from_secs(unit * amount))
}

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

    let id = IdExtractor::take_from(&mut state).id;
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

        let ttl = TtlExtractor::take_from(&mut state).ttl;
        let expiry = std::time::SystemTime::now() + ttl;

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
                    .put(data, expiry)
                    .map(|key| {
                        let mut response = key.encode().into_response(&state);
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

#[cfg(test)]
mod tests {
    #[test]
    fn can_deserialize_ttl() {
        use super::convert_str_to_duration;

        {
            let ttl = "1m";
            let duration = convert_str_to_duration(ttl).unwrap();
            assert_eq!(duration, std::time::Duration::from_secs(60));
        }
        {
            let ttl = "00009999m";
            let duration = convert_str_to_duration(ttl).unwrap();
            assert_eq!(duration, std::time::Duration::from_secs(9999 * 60));
        }
        {
            let ttl = "2h";
            let duration = convert_str_to_duration(ttl).unwrap();
            assert_eq!(duration, std::time::Duration::from_secs(7200));
        }
        {
            let ttl = "7d";
            let duration = convert_str_to_duration(ttl).unwrap();
            assert_eq!(duration, std::time::Duration::from_secs(7 * 24 * 60 * 60));
        }
    }

    #[test]
    fn reject_empty_ttl() {
        use super::convert_str_to_duration;

        let ttl = "";
        if let Err(e) = convert_str_to_duration(ttl) {
            assert_eq!(e, "ttl is empty");
        } else {
            panic!();
        }
    }

    #[test]
    fn reject_unknown_ttl_unit() {
        use super::convert_str_to_duration;

        let ttl = "1t";
        if let Err(e) = convert_str_to_duration(ttl) {
            assert_eq!(e, "invalid duration unit: t");
        } else {
            panic!();
        }
    }

    #[test]
    fn reject_empty_ttl_amount() {
        use super::convert_str_to_duration;

        let ttl = "h";
        if let Err(e) = convert_str_to_duration(ttl) {
            assert_eq!(
                e,
                "could not parse amount: cannot parse integer from empty string"
            );
        } else {
            panic!();
        }
    }

    #[test]
    fn reject_non_numeric_ttl_amount() {
        use super::convert_str_to_duration;

        let ttl = "12a3h";
        if let Err(e) = convert_str_to_duration(ttl) {
            assert_eq!(e, "could not parse amount: invalid digit found in string");
        } else {
            panic!();
        }
    }
}
