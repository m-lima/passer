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
        log::info!("Serving secrets from memory");
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
    path: std::path::PathBuf,
}

impl std::ops::Drop for InFileSecret {
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

impl InFileSecret {
    const HEADER_SIZE: usize = 7 + 15;

    fn read(path: std::path::PathBuf) -> Option<Self> {
        use std::io::Read;

        const MAGIC_HEADER_LENGTH: usize = 7;

        let mut file = std::fs::File::open(&path).ok()?;

        let mut buffer = [0_u8; 23];
        file.read_exact(&mut buffer).ok()?;

        if b"passer\n" != &buffer[..MAGIC_HEADER_LENGTH] {
            return None;
        }

        if b'\n' != buffer[Self::HEADER_SIZE - 1] {
            return None;
        }

        let expiry = {
            let mut millis: u64 = 0;
            for c in &buffer[MAGIC_HEADER_LENGTH + 1..Self::HEADER_SIZE - 1] {
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

    fn write(&self, data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        use std::io::Write;

        let mut file = std::fs::File::create(&self.path)?;

        let epoch_millis = self
            .expiry
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis();

        file.write_all(format!("passer\n{:014}\n", epoch_millis).as_bytes())?;
        file.write_all(data)?;
        Ok(())
    }

    fn expired(&self) -> bool {
        self.expiry <= std::time::SystemTime::now()
    }
}

impl InFile {
    const MAX_STORE_SIZE: u64 = MAX_SECRET_SIZE * 30;

    pub fn new(path: std::path::PathBuf) -> Self {
        log::info!("Serving secrets from file system");
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
            != 43
        {
            return None;
        }

        // Is it a valid file?
        let secret = InFileSecret::read(entry.path())?;

        Some((id, secret))
    }
}

unsafe impl Send for InFile {}

impl Store for InFile {
    fn refresh(&mut self) {
        self.secrets.retain(|_, secret| !secret.expired());
    }

    fn size(&self) -> u64 {
        self.secrets.values().map(|s| s.size).sum()
    }

    #[inline]
    fn max_size(&self) -> u64 {
        Self::MAX_STORE_SIZE
    }

    fn put(&mut self, expiry: std::time::SystemTime, data: Vec<u8>) -> Result<String, Error> {
        let (id, path) = loop {
            let id = new_id();
            if !self.secrets.contains_key(&id) {
                let path = self.path.join(&id);
                if !path.exists() {
                    break (id, path);
                }
            }
        };

        let size = (data.len() + InFileSecret::HEADER_SIZE) as u64;

        let mut secret = InFileSecret { expiry, path, size };

        if let Err(e) = secret.write(&data) {
            log::warn!(
                "Could not create secret file [{}]({}): {}",
                id,
                secret.path.display(),
                e
            );

            secret.expiry = std::time::UNIX_EPOCH;
            return Err(Error::Unknown(e));
        }

        self.secrets.insert(id.clone(), secret);
        Ok(id)
    }

    fn get(&mut self, id: &str) -> Result<Vec<u8>, Error> {
        use std::io::Read;

        let mut secret = self.secrets.remove(id).ok_or(Error::SecretNotFound)?;
        secret.expiry = std::time::UNIX_EPOCH;

        let mut file =
            std::fs::File::open(&secret.path).map_err(|e| Error::Unknown(Box::new(e)))?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| Error::Unknown(Box::new(e)))?;

        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::InFile;

    struct TempDir(std::path::PathBuf);

    impl std::ops::Drop for TempDir {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    #[test]
    fn scan_directory() {
        let store = InFile::new(std::path::PathBuf::from("res/test/store/scan"));

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
            std::fs::create_dir(&path.0).unwrap();
            let mut old_file = std::fs::File::create(path.0.join(OLD_FILE_NAME)).unwrap();

            old_file.write_all(b"passer\n").unwrap();
            old_file.write_all(b"00000000000001\n").unwrap();
            old_file.write_all(b"old_file\n").unwrap();
        }

        let mut store = InFile::new(path.0.clone());

        assert_eq!(store.secrets.len(), 1);
        assert!(store.secrets.contains_key(OLD_FILE_NAME));

        store.refresh();
        assert!(!store.secrets.contains_key(OLD_FILE_NAME));
        assert!(!path.0.join(OLD_FILE_NAME).exists());
    }

    #[test]
    fn expiry() {
        const EXPIRY_FILE_NAME: &str = "expiryfilelJSjJ6dUFBYlJVLXFfUmRzMVRTSEJEMHpwM3ppaEtON21Hcw";
        let path = TempDir(std::path::PathBuf::from("res/test/store/expiry"));

        {
            use std::io::Write;
            std::fs::create_dir(&path.0).unwrap();
            let mut old_file = std::fs::File::create(path.0.join(EXPIRY_FILE_NAME)).unwrap();

            old_file.write_all(b"passer\n").unwrap();
            old_file.write_all(b"01234567890123\n").unwrap();
            old_file.write_all(b"expiry\n").unwrap();
        }

        let store = InFile::new(path.0.clone());
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
        assert!(!path.0.exists());

        InFile::new(path.0.clone());
        assert!(path.0.exists());
        assert!(path.0.is_dir());
    }

    #[test]
    fn put() {
        use super::Store;
        let path = TempDir(std::path::PathBuf::from("res/test/store/put"));

        let mut store = InFile::new(path.0.clone());
        let data: Vec<u8> = b"test"[..].into();
        let id = store
            .put(
                std::time::SystemTime::now()
                    .checked_add(std::time::Duration::from_secs(1))
                    .unwrap(),
                data,
            )
            .unwrap();

        assert_eq!(id.len(), 43);
        assert!(path.0.join(&id).exists());
        assert!(path.0.join(&id).is_file());
    }

    #[test]
    fn get() {
        use super::Store;
        let path = TempDir(std::path::PathBuf::from("res/test/store/get"));

        let mut store = InFile::new(path.0.clone());
        let data: Vec<u8> = b"test"[..].into();
        let id = store
            .put(
                std::time::SystemTime::now()
                    .checked_add(std::time::Duration::from_secs(1))
                    .unwrap(),
                data,
            )
            .unwrap();

        let result = store.get(&id).unwrap();

        assert!(!path.0.join(&id).exists());
        assert_eq!(&result[..7], b"passer\n");
        assert_eq!(result[7 + 14], b'\n');
        assert_eq!(&result[7 + 15..], b"test");
    }

    #[test]
    fn refresh() {
        use super::Store;
        let path = TempDir(std::path::PathBuf::from("res/test/store/refresh"));

        let mut store = InFile::new(path.0.clone());
        let data: Vec<u8> = b"test"[..].into();
        let id = store
            .put(
                std::time::SystemTime::now()
                    .checked_add(std::time::Duration::from_millis(50))
                    .unwrap(),
                data,
            )
            .unwrap();

        assert!(path.0.join(&id).exists());
        assert!(path.0.join(&id).is_file());
        std::thread::sleep(std::time::Duration::from_millis(200));

        store.refresh();
        assert!(!path.0.join(&id).exists());
    }

    #[test]
    fn size() {
        use super::Store;
        let path = TempDir(std::path::PathBuf::from("res/test/store/size"));

        let mut store = InFile::new(path.0.clone());
        let data: Vec<u8> = b"test"[..].into();
        let id = store
            .put(
                std::time::SystemTime::now()
                    .checked_add(std::time::Duration::from_secs(1))
                    .unwrap(),
                data,
            )
            .unwrap();

        assert_eq!(store.size(), 7 + 15 + 4);
        assert_eq!(path.0.join(&id).metadata().unwrap().len(), 7 + 15 + 4);
    }
}
