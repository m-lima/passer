use super::error::Error;
use super::middleware;
use super::store;

#[derive(serde::Deserialize, gotham_derive::StateData, gotham_derive::StaticResponseExtender)]
pub struct IdExtractor {
    #[serde(deserialize_with = "id_deserializer")]
    id: store::Id,
}

fn id_deserializer<'de, D>(deserializer: D) -> Result<store::Id, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct IdVisitor;

    impl<'de> serde::de::Visitor<'de> for IdVisitor {
        type Value = store::Id;

        fn expecting(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            fmt.write_str("a 32-bit unsigned integer base64 encoded")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            store::Id::decode(value).map_err(|e| serde::de::Error::custom(e.to_string()))
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
            return Err(format!("invalid duration unit: {unit}"));
        }
        None => {
            return Err(String::from("ttl is empty"));
        }
    };

    let amount = value[..value.len() - 1]
        .parse::<u64>()
        .map_err(|e| format!("could not parse amount: {e}"))?;
    Ok(std::time::Duration::from_secs(unit * amount))
}

#[derive(Clone)]
pub struct Index(gotham::handler::DirHandler, gotham::handler::FileHandler);

impl Index {
    pub fn new(root: std::path::PathBuf, index: std::path::PathBuf) -> Self {
        use gotham::handler;
        Self(
            handler::DirHandler::new(handler::FileOptions::from(root)),
            handler::FileHandler::new(handler::FileOptions::from(index)),
        )
    }
}

impl gotham::handler::NewHandler for Index {
    type Instance = Self;

    fn new_handler(&self) -> gotham::anyhow::Result<Self::Instance> {
        Ok(self.clone())
    }
}

impl gotham::handler::Handler for Index {
    fn handle(
        self,
        state: gotham::state::State,
    ) -> std::pin::Pin<Box<gotham::handler::HandlerFuture>> {
        Box::pin(async {
            match self.0.handle(state).await {
                Ok(response) => Ok(response),
                Err((state, _)) => self.1.handle(state).await,
            }
        })
    }
}

pub fn get(mut state: gotham::state::State) -> std::pin::Pin<Box<gotham::handler::HandlerFuture>> {
    Box::pin(async {
        use gotham::handler::IntoResponse;
        use gotham::state::FromState;

        let id = IdExtractor::take_from(&mut state).id;
        let store = middleware::Store::borrow_mut_from(&mut state);

        match store.get(&id).map_err(Error::from) {
            Ok(r) => {
                let response = r.into_response(&state);
                Ok((state, response))
            }
            Err(e) => Err((state, e.into_handler_error())),
        }
    })
}

pub fn post(mut state: gotham::state::State) -> std::pin::Pin<Box<gotham::handler::HandlerFuture>> {
    // TODO: Todo one try-block has landed
    async fn internal(
        state: &mut gotham::state::State,
    ) -> Result<gotham::hyper::Response<gotham::hyper::Body>, Error> {
        use gotham::handler::IntoResponse;
        use gotham::state::FromState;
        use std::convert::TryFrom;

        let request_length = gotham::hyper::HeaderMap::borrow_from(state)
            .get(gotham::hyper::header::CONTENT_LENGTH)
            .and_then(|len| len.to_str().ok())
            .and_then(|len| len.parse::<usize>().ok())
            .ok_or(Error::ContentLengthMissing)?;

        if request_length == 0 {
            return Err(Error::NothingToInsert);
        } else if u64::try_from(request_length).map_err(|_| Error::PayloadTooLarge)?
            > store::MAX_SECRET_SIZE
        {
            return Err(Error::PayloadTooLarge);
        }

        // Hyper reads up to Content-Length. No need for chunk-wise verification
        // TODO: Is this needed behind nginx?
        let body = tokio::time::timeout(
            std::time::Duration::from_secs(10),
            gotham::hyper::body::to_bytes(gotham::hyper::Body::borrow_mut_from(state)),
        )
        .await
        .map_err(|_| Error::ReadTimeout)?
        .map_err(Error::Hyper)?;

        let ttl = TtlExtractor::take_from(state).ttl;
        let expiry = std::time::SystemTime::now() + ttl;

        let store = middleware::Store::borrow_mut_from(state);
        store
            .put(body.to_vec(), expiry)
            .map(|key| {
                let mut response = key.encode().into_response(state);
                *response.status_mut() = gotham::hyper::StatusCode::CREATED;
                response
            })
            .map_err(Error::from)
    }

    Box::pin(async {
        match internal(&mut state).await {
            Ok(r) => Ok((state, r)),
            Err(e) => Err((state, e.into_handler_error())),
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
