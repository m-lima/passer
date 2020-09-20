mod in_file;
mod in_memory;

const MAX_SECRET_SIZE: u64 = 110 * 1024 * 1024;

#[derive(Debug)]
pub enum Error {
    TooLarge,
    StoreFull,
    SecretNotFound,
    Generic(String),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge => write!(fmt, "payload too large"),
            Self::StoreFull => write!(fmt, "store is full"),
            Self::SecretNotFound => write!(fmt, "secret not found"),
            Self::Generic(msg) => write!(fmt, "{}", msg),
        }
    }
}

pub trait Store {
    fn refresh(&mut self);
    fn put(&mut self, expiry: std::time::SystemTime, data: Vec<u8>) -> Result<String, Error>;
    fn get(&mut self, id: &str) -> Result<Vec<u8>, Error>;
}

pub fn in_memory() -> impl Store {
    in_memory::Store::new()
}

pub fn in_file(path: std::path::PathBuf) -> impl Store {
    in_file::Store::new(path)
}

fn new_id() -> String {
    use rand::Rng;
    let id = rand::thread_rng().gen::<[u8; 32]>();
    base64::encode_config(&id, base64::URL_SAFE_NO_PAD)
}
