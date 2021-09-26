use super::error::Error;
use super::middleware;
use super::store::Id;

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

#[derive(Clone)]
pub struct Index(
    gotham::handler::assets::DirHandler,
    gotham::handler::assets::FileHandler,
);

impl Index {
    pub fn new(root: std::path::PathBuf, index: std::path::PathBuf) -> Self {
        use gotham::handler::assets;
        Self(
            assets::DirHandler::new(assets::FileOptions::from(root)),
            assets::FileHandler::new(assets::FileOptions::from(index)),
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
    Box::pin(async {
        use gotham::handler::IntoResponse;
        use gotham::hyper::{body, Body};
        use gotham::state::FromState;

        let ttl = TtlExtractor::take_from(&mut state).ttl;
        let expiry = std::time::SystemTime::now() + ttl;

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
                        *response.status_mut() = gotham::hyper::StatusCode::CREATED;
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
