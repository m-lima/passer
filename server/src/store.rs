#[derive(Debug)]
pub enum Error {
    TooLarge,
    StoreFull,
    InvalidExpiry,
    BadSecret(String),
    IO(std::io::Error),
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooLarge => write!(fmt, "payload too large"),
            Self::StoreFull => write!(fmt, "store is full"),
            Self::InvalidExpiry => write!(fmt, "invalid expiry"),
            Self::BadSecret(e) => write!(fmt, "bad secret: {}", e),
            Self::IO(e) => write!(fmt, "io error: {}", e),
        }
    }
}

impl std::convert::From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::IO(e)
    }
}

impl std::convert::From<SecretError> for Error {
    fn from(e: SecretError) -> Self {
        match e {
            SecretError::IO(e) => Self::IO(e),
            SecretError::InvalidExpiry => Self::InvalidExpiry,
            e => Self::BadSecret(e.to_string()),
        }
    }
}

#[derive(Debug)]
enum SecretError {
    NotReadableFile,
    BadName,
    BadHeader,
    InvalidExpiry,
    IO(std::io::Error),
}

impl std::error::Error for SecretError {}

impl std::fmt::Display for SecretError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotReadableFile => write!(fmt, "not a readable file"),
            Self::BadName => write!(fmt, "bad name"),
            Self::BadHeader => write!(fmt, "bad header"),
            Self::InvalidExpiry => write!(fmt, "invalid expiry"),
            Self::IO(e) => write!(fmt, "io error: {}", e),
        }
    }
}

impl std::convert::From<std::io::Error> for SecretError {
    fn from(e: std::io::Error) -> Self {
        Self::IO(e)
    }
}

pub struct Store {
    secrets: std::collections::HashMap<String, Secret>,
    path: std::path::PathBuf,
}

impl Store {
    const MAX_SIZE: u64 = Secret::MAX_SIZE * 30;

    pub fn new(path: std::path::PathBuf) -> Self {
        log::info!("Serving secrets from file system");
        if path.exists() {
            log::info!("Scanning store directory at {}", path.display());
            let reader = std::fs::read_dir(&path).expect("Could not open store directory");

            let secrets = reader
                .filter_map(Result::ok)
                .inspect(|file| log::info!("Scanning {}", file.path().display()))
                .map(Self::map_secret)
                .inspect(|secret| {
                    if let Err(e) = secret {
                        match e {
                            SecretError::IO(e) => log::warn!("Ignoring secret: {}", e),
                            e => log::info!("Ignoring secret: {}", e),
                        }
                    }
                })
                .filter_map(Result::ok)
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

            Self { secrets, path }
        } else {
            log::info!(
                "Store directory does not exist. Creating {}",
                path.display()
            );
            std::fs::create_dir(&path).expect("Could not create store directory");

            Self {
                secrets: std::collections::HashMap::<_, _>::new(),
                path,
            }
        }
    }

    pub fn refresh(&mut self) {
        self.secrets.retain(|_, secret| !secret.expired());
    }

    pub fn size(&self) -> u64 {
        self.secrets.values().map(|s| s.size).sum()
    }

    pub fn put(&mut self, expiry: std::time::SystemTime, data: &[u8]) -> Result<String, Error> {
        let size = (data.len() + Secret::HEADER_SIZE) as u64;

        if size > Secret::MAX_SIZE {
            return Err(Error::TooLarge);
        }

        if self.size() + size > Self::MAX_SIZE {
            return Err(Error::StoreFull);
        }

        let (id, path) = loop {
            let id = new_id();
            if !self.secrets.contains_key(&id) {
                let path = self.path.join(&id);
                if !path.exists() {
                    break (id, path);
                }
            }
        };

        let mut secret = Secret { expiry, path, size };

        if let Err(e) = secret.write(&data) {
            log::warn!(
                "Could not create secret file [{}]({}): {}",
                id,
                secret.path.display(),
                e
            );

            secret.expiry = std::time::UNIX_EPOCH;
            return Err(e.into());
        }

        self.secrets.insert(id.clone(), secret);
        Ok(id)
    }

    pub fn get(&mut self, id: &str) -> Result<Option<Vec<u8>>, Error> {
        use std::io::Read;
        use std::io::Seek;

        let mut secret = match self.secrets.remove(id) {
            Some(secret) => secret,
            None => return Ok(None),
        };
        secret.expiry = std::time::UNIX_EPOCH;

        let mut file = std::fs::File::open(&secret.path)?;

        let mut buffer = Vec::new();
        file.seek(std::io::SeekFrom::Start(Secret::HEADER_SIZE as u64))?;
        file.read_to_end(&mut buffer)?;

        Ok(Some(buffer))
    }

    // Allowed for readability
    #[allow(clippy::needless_pass_by_value)]
    fn map_secret(entry: std::fs::DirEntry) -> Result<(String, Secret), SecretError> {
        // Is it a file?
        if !entry.file_type()?.is_file() {
            return Err(SecretError::NotReadableFile);
        }

        let id = entry
            .file_name()
            .into_string()
            .map_err(|_| SecretError::BadName)?;

        // Does the name fit expectations?
        if base64::decode_config(&id, base64::URL_SAFE_NO_PAD)
            .map_err(|_| SecretError::BadName)?
            .len()
            != 43
        {
            return Err(SecretError::BadName);
        }

        // Is it a valid file?
        let secret = Secret::read(entry.path())?;

        Ok((id, secret))
    }
}

struct Secret {
    expiry: std::time::SystemTime,
    path: std::path::PathBuf,
    size: u64,
}

impl Secret {
    const MAX_SIZE: u64 = 110 * 1024 * 1024;

    const HEADER_SIZE: usize = Self::MAGIC_NUMBER_LENGTH + Self::EXPIRY_LENGTH;
    const MAGIC_NUMBER_LENGTH: usize = 6 + 1;
    const EXPIRY_LENGTH: usize = 14 + 1;

    fn read(path: std::path::PathBuf) -> Result<Self, SecretError> {
        use std::io::Read;

        let mut file = std::fs::File::open(&path)?;

        let mut buffer = [0_u8; Self::HEADER_SIZE];
        file.read_exact(&mut buffer)?;

        if b"passer\n" != &buffer[..Self::MAGIC_NUMBER_LENGTH] {
            return Err(SecretError::BadHeader);
        }

        if b'\n' != buffer[Self::HEADER_SIZE - 1] {
            return Err(SecretError::BadHeader);
        }

        let expiry = {
            let mut millis: u64 = 0;
            for c in &buffer[Self::MAGIC_NUMBER_LENGTH..Self::HEADER_SIZE - 1] {
                if *c < b'0' || *c > b'9' {
                    return Err(SecretError::InvalidExpiry);
                }

                millis *= 10;
                millis += u64::from(*c - b'0');
            }
            std::time::UNIX_EPOCH
                .checked_add(std::time::Duration::from_millis(millis))
                .ok_or(SecretError::InvalidExpiry)?
        };

        let size = path.metadata()?.len();

        Ok(Self { expiry, size, path })
    }

    fn write(&self, data: &[u8]) -> Result<(), SecretError> {
        use std::io::Write;

        let mut file = std::fs::File::create(&self.path)?;

        let epoch_millis = self
            .expiry
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| SecretError::InvalidExpiry)?
            .as_millis();

        file.write_all(format!("passer\n{:014}\n", epoch_millis).as_bytes())?;
        file.write_all(data)?;
        Ok(())
    }

    fn expired(&self) -> bool {
        self.expiry <= std::time::SystemTime::now()
    }
}

impl std::ops::Drop for Secret {
    fn drop(&mut self) {
        if self.expired() {
            if let Err(e) = std::fs::remove_file(&self.path) {
                log::warn!(
                    "Could not delete untracked secret file {}: {}",
                    self.path.display(),
                    e
                );
            }
        }
    }
}

fn new_id() -> String {
    use rand::Rng;
    let id = rand::thread_rng().gen::<[u8; 32]>();
    base64::encode_config(&id, base64::URL_SAFE_NO_PAD)
}

#[cfg(test)]
mod tests {
    use super::Store;

    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn get(&self) -> &std::path::PathBuf {
            &self.0
        }

        fn clone(&self) -> std::path::PathBuf {
            self.0.clone()
        }
    }

    impl std::ops::Drop for TempDir {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn scan_directory() {
        let store = Store::new(std::path::PathBuf::from("res/test/store/scan"));

        assert_eq!(store.secrets.len(), 2);
        assert!(store
            .secrets
            .contains_key("file1dNzbGlJSjJ6dUFBYlJVLXFfUmRzMVRTSEJEMHpwM3ppaEtON21Hcw"));
        assert!(store
            .secrets
            .contains_key("file2ddLczRYTWR5T3FzdWUtUnR3a1RNbE9HTVBzRHZRSEliNzhFNUlOaw"));
    }

    #[test]
    fn accept_old_files() {
        use super::Store;

        const OLD_FILE_NAME: &str = "oldfile1bGlJSjJ6dUFBYlJVLXFfUmRzMVRTSEJEMHpwM3ppaEtON21Hcw";
        let path = TempDir(std::path::PathBuf::from("res/test/store/scan_old"));

        {
            use std::io::Write;
            std::fs::create_dir(path.get()).unwrap();
            let mut old_file = std::fs::File::create(path.get().join(OLD_FILE_NAME)).unwrap();

            old_file.write_all(b"passer\n").unwrap();
            old_file.write_all(b"00000000000001\n").unwrap();
            old_file.write_all(b"old_file\n").unwrap();
        }

        let mut store = Store::new(path.clone());

        assert_eq!(store.secrets.len(), 1);
        assert!(store.secrets.contains_key(OLD_FILE_NAME));

        store.refresh();
        assert!(!store.secrets.contains_key(OLD_FILE_NAME));
        assert!(!path.get().join(OLD_FILE_NAME).exists());
    }

    #[test]
    fn expiry() {
        const EXPIRY_FILE_NAME: &str = "expiryfilelJSjJ6dUFBYlJVLXFfUmRzMVRTSEJEMHpwM3ppaEtON21Hcw";
        let path = TempDir(std::path::PathBuf::from("res/test/store/expiry"));

        {
            use std::io::Write;
            std::fs::create_dir(path.get()).unwrap();
            let mut old_file = std::fs::File::create(path.get().join(EXPIRY_FILE_NAME)).unwrap();

            old_file.write_all(b"passer\n").unwrap();
            old_file.write_all(b"01234567890123\n").unwrap();
            old_file.write_all(b"expiry\n").unwrap();
        }

        let store = Store::new(path.clone());
        let secret = store.secrets.get(EXPIRY_FILE_NAME).unwrap();
        assert_eq!(
            secret.expiry,
            std::time::UNIX_EPOCH
                .checked_add(std::time::Duration::from_millis(1_234_567_890_123))
                .unwrap()
        );
    }

    #[test]
    fn create_directory() {
        let path = TempDir(std::path::PathBuf::from("res/test/store/create_directory"));
        assert!(!path.get().exists());

        Store::new(path.clone());
        assert!(path.get().exists());
        assert!(path.get().is_dir());
    }

    #[test]
    fn put() {
        use super::Store;
        let path = TempDir(std::path::PathBuf::from("res/test/store/put"));

        let mut store = Store::new(path.clone());
        let data: Vec<u8> = b"test"[..].into();
        let id = store
            .put(
                std::time::SystemTime::now()
                    .checked_add(std::time::Duration::from_secs(1))
                    .unwrap(),
                &data,
            )
            .unwrap();

        assert_eq!(id.len(), 43);
        assert!(path.get().join(&id).exists());
        assert!(path.get().join(&id).is_file());
    }

    #[test]
    fn get() {
        use super::Store;
        let path = TempDir(std::path::PathBuf::from("res/test/store/get"));

        let mut store = Store::new(path.clone());
        let data: Vec<u8> = b"test"[..].into();
        let id = store
            .put(
                std::time::SystemTime::now()
                    .checked_add(std::time::Duration::from_secs(1))
                    .unwrap(),
                &data,
            )
            .unwrap();

        let result = store.get(&id).unwrap().unwrap();

        assert!(!path.get().join(&id).exists());
        assert_eq!(&result[..], b"test");
    }

    #[test]
    fn refresh() {
        use super::Store;
        let path = TempDir(std::path::PathBuf::from("res/test/store/refresh"));

        let mut store = Store::new(path.clone());
        let data: Vec<u8> = b"test"[..].into();
        let id = store
            .put(
                std::time::SystemTime::now()
                    .checked_add(std::time::Duration::from_millis(50))
                    .unwrap(),
                &data,
            )
            .unwrap();

        assert!(path.get().join(&id).exists());
        assert!(path.get().join(&id).is_file());
        std::thread::sleep(std::time::Duration::from_millis(200));

        store.refresh();
        assert!(!path.get().join(&id).exists());
    }

    #[test]
    fn size() {
        use super::Store;
        let path = TempDir(std::path::PathBuf::from("res/test/store/size"));

        let mut store = Store::new(path.clone());
        let data: Vec<u8> = b"test"[..].into();
        let id = store
            .put(
                std::time::SystemTime::now()
                    .checked_add(std::time::Duration::from_secs(1))
                    .unwrap(),
                &data,
            )
            .unwrap();

        assert_eq!(store.size(), 7 + 15 + 4);
        assert_eq!(path.get().join(&id).metadata().unwrap().len(), 7 + 15 + 4);
    }
}
