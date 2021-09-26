mod in_file;
mod in_memory;

const MAX_SECRET_SIZE: u64 = 110 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("payload too large")]
    TooLarge,
    #[error("store full")]
    StoreFull,
    #[error("secret not found")]
    SecretNotFound,
    #[error("invalid id: {0}")]
    InvalidId(base64::DecodeError),
    #[error("{0}")]
    Generic(String),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Id([u8; 32]);

impl Id {
    fn new() -> Self {
        use rand::Rng;
        Self(rand::thread_rng().gen())
    }

    pub fn decode<S: AsRef<str>>(string: S) -> Result<Self, Error> {
        let mut id = [0_u8; 32];
        let size = base64::decode_config_slice(
            string.as_ref().as_bytes(),
            base64::URL_SAFE_NO_PAD,
            &mut id,
        )
        .map_err(Error::InvalidId)?;

        if size == 32 {
            Ok(Self(id))
        } else {
            Err(Error::InvalidId(base64::DecodeError::InvalidLength))
        }
    }

    pub fn encode(&self) -> String {
        base64::encode_config(&self.0, base64::URL_SAFE_NO_PAD)
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{}", self.encode())
    }
}

pub trait Store {
    fn refresh(&mut self);
    fn put(&mut self, expiry: std::time::SystemTime, data: Vec<u8>) -> Result<Id, Error>;
    fn get(&mut self, id: &Id) -> Result<Vec<u8>, Error>;
}

pub fn in_memory() -> impl Store {
    in_memory::Store::new()
}

pub fn in_file(path: std::path::PathBuf) -> impl Store {
    in_file::Store::new(path)
}

#[cfg(test)]
mod test {
    use super::Id;

    #[test]
    fn id_size() {
        let id = Id::new();
        let id_string = id.encode();
        assert_eq!(id_string.len(), 43);
    }

    #[test]
    fn id_roundtrip() {
        let id = Id::new();
        let id_string = id.encode();
        let id_recovered = Id::decode(&id_string).unwrap();

        assert_eq!(id, id_recovered);
    }

    #[test]
    fn reject_invalid_id() {
        let id = Id::new();
        let id_string = id.encode();

        if let Err(super::Error::InvalidId(_)) = Id::decode(&id_string[1..]) {
        } else {
            panic!();
        }
    }

    #[test]
    fn accept_valid_id() {
        let _ = Id::decode("VhmE7GuDMxsrCM6Mu8zvBX5Hr8_COegK4EomGENCRCQ").unwrap();
    }
}
