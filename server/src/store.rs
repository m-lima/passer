pub const MAX_SECRET_SIZE: u64 = 110 * 1024 * 1024;

#[derive(Debug)]
pub enum Error {
    SecretNotFound,
    Unknown(Box<dyn std::error::Error>),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SecretNotFound => write!(fmt, "secret not found"),
            Self::Unknown(e) => write!(fmt, "internal error: {}", e),
        }
    }
}

pub trait Store {
    fn refresh(&mut self);
    fn size(&self) -> u64;
    fn max_size(&self) -> u64;
    fn put(&mut self, expiry: std::time::SystemTime, secret: Vec<u8>) -> Result<String, Error>;
    fn get(&mut self, id: &str) -> Result<Vec<u8>, Error>;
}

fn new_id() -> String {
    use rand::Rng;
    let id = rand::thread_rng().gen::<[u8; 32]>();
    base64::encode_config(&id, base64::URL_SAFE_NO_PAD)
}

struct InMemorySecret {
    expiry: std::time::SystemTime,
    secret: Vec<u8>,
}

pub struct InMemory {
    secrets: std::collections::HashMap<String, InMemorySecret>,
}

impl InMemory {
    const MAX_STORE_SIZE: u64 = MAX_SECRET_SIZE * 10;

    pub fn new() -> Self {
        Self {
            secrets: std::collections::HashMap::<_, _>::new(),
        }
    }
}

unsafe impl Send for InMemory {}

impl Store for InMemory {
    fn refresh(&mut self) {
        self.secrets
            .retain(|_, secret| secret.expiry > std::time::SystemTime::now());
    }

    fn size(&self) -> u64 {
        self.secrets
            .values()
            .map(|s| s.secret.len())
            .fold(0, |a, c| a + (c as u64))
    }

    #[inline]
    fn max_size(&self) -> u64 {
        Self::MAX_STORE_SIZE
    }

    fn put(&mut self, expiry: std::time::SystemTime, secret: Vec<u8>) -> Result<String, Error> {
        let id = loop {
            let id = new_id();
            if !self.secrets.contains_key(&id) {
                break id;
            }
        };

        self.secrets
            .insert(id.clone(), InMemorySecret { expiry, secret });
        Ok(id)
    }

    fn get(&mut self, id: &str) -> Result<Vec<u8>, Error> {
        self.secrets
            .remove(id)
            .map(|s| s.secret)
            .ok_or(Error::SecretNotFound)
    }
}

struct InFileSecret {
    expiry: std::time::SystemTime,
    path: std::path::PathBuf,
    size: u64,
}

pub struct InFile {
    secrets: std::collections::HashMap<String, InFileSecret>,
    size: u64,
}

impl InFileSecret {
    fn read(path: std::path::PathBuf) -> Option<Self> {
        use std::io::Read;

        const MAGIC_HEADER_LENGTH: usize = 7;
        const EXPIRY_LENGTH: usize = 15;

        let mut file = std::fs::File::open(&path).ok()?;

        let mut buffer = [0_u8; 23];
        file.read_exact(&mut buffer).ok()?;

        if b"passer\n" != &buffer[..MAGIC_HEADER_LENGTH] {
            return None;
        }

        if b'\n' != buffer[MAGIC_HEADER_LENGTH + EXPIRY_LENGTH] {
            return None;
        }

        let expiry = {
            let mut millis: u64 = 0;
            for c in &buffer[MAGIC_HEADER_LENGTH..EXPIRY_LENGTH - 1] {
                if *c < b'0' || *c > b'9' {
                    return None;
                }

                millis *= 10;
                millis += u64::from(*c - b'0');
            }
            std::time::UNIX_EPOCH.checked_add(std::time::Duration::from_millis(millis))?
        };

        let size = path.metadata().ok()?.len();

        Some(Self { expiry, size, path })
    }

    fn write(&self) -> Result<(), Box<dyn std::error::Error>> {
        use std::io::Write;

        let mut file = std::fs::File::create(&self.path)?;

        let epoch_millis = self
            .expiry
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis();

        file.write_all(format!("passer\n{:014}\n", epoch_millis).as_bytes())?;
        Ok(())
    }
}

impl InFile {
    const MAX_STORE_SIZE: u64 = MAX_SECRET_SIZE * 30;

    pub fn new(path: &std::path::PathBuf) -> Self {
        if path.exists() {
            log::info!("Scanning store directory at {}", path.display());
            let reader = std::fs::read_dir(&path).expect("Could not open store directory");

            let secrets = reader
                .filter_map(Result::ok)
                .filter_map(Self::map_secret)
                .inspect(|secret| {
                    log::info!(
                        "Tracking secret {} for {}s",
                        secret.1.path.display(),
                        secret
                            .1
                            .expiry
                            .duration_since(std::time::SystemTime::now())
                            .map(|d| d.as_secs())
                            .unwrap_or(0)
                    )
                })
                .collect::<std::collections::HashMap<_, _>>();

            let size = secrets.values().map(|s| s.size).sum();

            Self { secrets, size }
        } else {
            log::info!(
                "Store directory does not exist. Creating {}",
                path.display()
            );
            std::fs::create_dir(&path).expect("Could not create store directory");

            Self {
                secrets: std::collections::HashMap::<_, _>::new(),
                size: 0,
            }
        }
    }

    // Allowed for readability
    #[allow(clippy::needless_pass_by_value)]
    fn map_secret(entry: std::fs::DirEntry) -> Option<(String, InFileSecret)> {
        // Is it a file?
        if !entry.file_type().ok()?.is_file() {
            return None;
        }

        let id = entry.file_name().into_string().ok()?;

        // Does the name fit expectations?
        if base64::decode_config(&id, base64::URL_SAFE_NO_PAD)
            .ok()?
            .len()
            != 32
        {
            return None;
        }

        // Is it a valid file?
        let secret = InFileSecret::read(entry.path())?;

        Some((id, secret))
    }

    fn remove(&mut self, id: &str) {
        if let Some(secret) = self.secrets.remove(id) {
            if let Err(e) = std::fs::remove_file(&secret.path) {
                log::warn!(
                    "Could not delete secret file {}: {}. Untracking",
                    secret.path.display(),
                    e
                );
            }

            self.size -= secret.size;
        } else {
            log::warn!("Could not untrack {}", id);
        }
    }
}

unsafe impl Send for InFile {}

impl Store for InFile {
    fn refresh(&mut self) {
        let for_removal = self
            .secrets
            .iter()
            .filter_map(|(id, secret)| {
                if secret.expiry <= std::time::SystemTime::now() {
                    Some(id.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for id in &for_removal {
            self.remove(id);
        }
    }

    fn size(&self) -> u64 {
        self.size
    }

    #[inline]
    fn max_size(&self) -> u64 {
        Self::MAX_STORE_SIZE
    }

    fn put(&mut self, expiry: std::time::SystemTime, secret: Vec<u8>) -> Result<String, Error> {
        let (id, path) = loop {
            let id = new_id();
            if !self.secrets.contains_key(&id) {
                let path = std::path::PathBuf::from(&id);
                if !path.exists() {
                    break (id, path);
                }
            }
        };

        let size = secret.len() as u64;

        let secret = InFileSecret { expiry, path, size };

        if let Err(e) = secret.write() {
            log::warn!(
                "Could not create secret file [{}]({}): {}",
                id,
                secret.path.display(),
                e
            );

            if let Err(e) = std::fs::remove_file(&secret.path) {
                log::warn!(
                    "Failed to clean up malformed secret file {}: {}",
                    secret.path.display(),
                    e
                );
            }

            return Err(Error::Unknown(e));
        }

        self.secrets.insert(id.clone(), secret);
        Ok(id)
    }

    fn get(&mut self, id: &str) -> Result<Vec<u8>, Error> {
        use std::io::Read;

        let secret = self.secrets.get(id).ok_or(Error::SecretNotFound)?;

        let mut file =
            std::fs::File::open(&secret.path).map_err(|e| Error::Unknown(Box::new(e)))?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| Error::Unknown(Box::new(e)))?;

        self.remove(id);

        Ok(buffer)
    }
}
