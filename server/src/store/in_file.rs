use super::Error;
use super::Id;

#[derive(Debug)]
pub enum InternalError {
    NotReadableFile,
    BadName,
    BadHeader,
    InvalidExpiry,
    IO(std::io::Error),
}

impl std::error::Error for InternalError {}

impl std::fmt::Display for InternalError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotReadableFile => write!(fmt, "not a readable file"),
            Self::BadName => write!(fmt, "file name does not match expected pattern"),
            Self::BadHeader => write!(fmt, "bad header"),
            Self::InvalidExpiry => write!(fmt, "invalid expiry"),
            Self::IO(e) => write!(fmt, "io error: {}", e),
        }
    }
}

impl std::convert::From<std::io::Error> for InternalError {
    fn from(e: std::io::Error) -> Self {
        Self::IO(e)
    }
}

impl std::convert::From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self::Generic(e.to_string())
    }
}

impl std::convert::Into<Error> for InternalError {
    fn into(self) -> Error {
        Error::Generic(self.to_string())
    }
}

pub struct Store {
    secrets: std::collections::HashMap<Id, Secret>,
    path: std::path::PathBuf,
}

impl Store {
    const MAX_SIZE: u64 = super::MAX_SECRET_SIZE * 30;

    pub fn new(path: std::path::PathBuf) -> Self {
        log::info!("Serving secrets from file system");
        if path.exists() {
            log::info!("Scanning store directory at {}", path.display());
            let reader = std::fs::read_dir(&path).expect("Could not open store directory");

            let secrets = reader
                .filter_map(Result::ok)
                .inspect(|file| log::info!("Scanning {}", file.path().display()))
                .map(Self::map_secret)
                .filter_map(|secret| match secret {
                    Ok(secret) => {
                        log::info!(
                            "Tracking secret {} for {}s",
                            secret.1.path.display(),
                            secret
                                .1
                                .expiry
                                .duration_since(std::time::SystemTime::now())
                                .map(|d| d.as_secs())
                                .unwrap_or(0)
                        );
                        Some(secret)
                    }
                    Err(InternalError::IO(e)) => {
                        log::warn!("Ignoring secret: {}", e);
                        None
                    }
                    Err(e) => {
                        log::info!("Ignoring secret: {}", e);
                        None
                    }
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
    fn map_secret(entry: std::fs::DirEntry) -> Result<(Id, Secret), InternalError> {
        // Is it a file?
        if !entry.file_type()?.is_file() {
            return Err(InternalError::NotReadableFile);
        }

        let id = Id::decode(
            entry
                .file_name()
                .into_string()
                .map_err(|_| InternalError::BadName)?,
        )
        .map_err(|_| InternalError::BadName)?;

        // Is it a valid file?
        let secret = Secret::read(entry.path())?;

        Ok((id, secret))
    }

    #[inline]
    fn size(&self) -> u64 {
        self.secrets.values().map(|s| s.size).sum()
    }
}

impl super::Store for Store {
    fn refresh(&mut self) {
        self.secrets.retain(|_, secret| !secret.expired());
    }

    fn put(&mut self, expiry: std::time::SystemTime, data: Vec<u8>) -> Result<Id, Error> {
        let size = (data.len() + Secret::HEADER_SIZE) as u64;

        if size > super::MAX_SECRET_SIZE {
            return Err(Error::TooLarge);
        }

        if self.size() + size > Self::MAX_SIZE {
            return Err(Error::StoreFull);
        }

        let (id, path) = loop {
            let id = Id::new();
            if !self.secrets.contains_key(&id) {
                let path = self.path.join(&id.encode());
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

        self.secrets.insert(id, secret);
        Ok(id)
    }

    fn get(&mut self, id: &Id) -> Result<Vec<u8>, Error> {
        use std::io::Read;
        use std::io::Seek;

        let mut secret = match self.secrets.remove(id) {
            Some(secret) => secret,
            None => return Err(Error::SecretNotFound),
        };
        secret.expiry = std::time::UNIX_EPOCH;

        let mut file = std::fs::File::open(&secret.path)?;

        let mut buffer = Vec::new();
        file.seek(std::io::SeekFrom::Start(Secret::HEADER_SIZE as u64))?;
        file.read_to_end(&mut buffer)?;

        Ok(buffer)
    }
}

struct Secret {
    expiry: std::time::SystemTime,
    path: std::path::PathBuf,
    size: u64,
}

impl Secret {
    const HEADER_SIZE: usize = Self::MAGIC_NUMBER_LENGTH + Self::EXPIRY_LENGTH;
    const MAGIC_NUMBER_LENGTH: usize = 6 + 1;
    const EXPIRY_LENGTH: usize = 14 + 1;

    fn read(path: std::path::PathBuf) -> Result<Self, InternalError> {
        use std::io::Read;

        let mut file = std::fs::File::open(&path)?;

        let mut buffer = [0_u8; Self::HEADER_SIZE];
        file.read_exact(&mut buffer)?;

        if b"passer\n" != &buffer[..Self::MAGIC_NUMBER_LENGTH] {
            return Err(InternalError::BadHeader);
        }

        if b'\n' != buffer[Self::HEADER_SIZE - 1] {
            return Err(InternalError::BadHeader);
        }

        let expiry = {
            let mut millis: u64 = 0;
            for c in &buffer[Self::MAGIC_NUMBER_LENGTH..Self::HEADER_SIZE - 1] {
                if *c < b'0' || *c > b'9' {
                    return Err(InternalError::InvalidExpiry);
                }

                millis *= 10;
                millis += u64::from(*c - b'0');
            }
            std::time::UNIX_EPOCH
                .checked_add(std::time::Duration::from_millis(millis))
                .ok_or(InternalError::InvalidExpiry)?
        };

        let size = path.metadata()?.len();

        Ok(Self { expiry, size, path })
    }

    fn write(&self, data: &[u8]) -> Result<(), InternalError> {
        use std::io::Write;

        let mut file = std::fs::File::create(&self.path)?;

        let epoch_millis = self
            .expiry
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| InternalError::InvalidExpiry)?
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

#[cfg(test)]
mod tests {
    use super::super::Id;
    use super::super::Store as Trait;
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
            .contains_key(&Id::decode("file_1____________________________________0").unwrap()));
        assert!(store
            .secrets
            .contains_key(&Id::decode("file_2____________________________________0").unwrap()));
    }

    #[test]
    fn accept_old_files() {
        const OLD_FILE_NAME: &str = "old_file__________________________________0";
        let old_id = Id::decode(OLD_FILE_NAME).unwrap();

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
        assert!(store.secrets.contains_key(&old_id));

        store.refresh();
        assert!(!store.secrets.contains_key(&old_id));
        assert!(!path.get().join(OLD_FILE_NAME).exists());
    }

    #[test]
    fn expiry() {
        const EXPIRY_FILE_NAME: &str = "expiry_file_______________________________0";
        let expiry_id = Id::decode(EXPIRY_FILE_NAME).unwrap();

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
        let secret = store.secrets.get(&expiry_id).unwrap();
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
        let path = TempDir(std::path::PathBuf::from("res/test/store/put"));

        let mut store = Store::new(path.clone());
        let data: Vec<u8> = b"test"[..].into();
        let id = store
            .put(
                std::time::SystemTime::now()
                    .checked_add(std::time::Duration::from_secs(1))
                    .unwrap(),
                data,
            )
            .unwrap()
            .encode();

        assert_eq!(id.len(), 43);
        assert!(path.get().join(&id).exists());
        assert!(path.get().join(&id).is_file());
    }

    #[test]
    fn get() {
        let path = TempDir(std::path::PathBuf::from("res/test/store/get"));

        let mut store = Store::new(path.clone());
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

        assert!(!path.get().join(&id.encode()).exists());
        assert_eq!(&result[..], b"test");
    }

    #[test]
    fn refresh() {
        let path = TempDir(std::path::PathBuf::from("res/test/store/refresh"));

        let mut store = Store::new(path.clone());
        let data: Vec<u8> = b"test"[..].into();
        let id = store
            .put(
                std::time::SystemTime::now()
                    .checked_add(std::time::Duration::from_millis(50))
                    .unwrap(),
                data,
            )
            .unwrap()
            .encode();

        assert!(path.get().join(&id).exists());
        assert!(path.get().join(&id).is_file());
        std::thread::sleep(std::time::Duration::from_millis(200));

        store.refresh();
        assert!(!path.get().join(&id).exists());
    }

    #[test]
    fn size() {
        let path = TempDir(std::path::PathBuf::from("res/test/store/size"));

        let mut store = Store::new(path.clone());
        let data: Vec<u8> = b"test"[..].into();
        let id = store
            .put(
                std::time::SystemTime::now()
                    .checked_add(std::time::Duration::from_secs(1))
                    .unwrap(),
                data,
            )
            .unwrap()
            .encode();

        assert_eq!(store.size(), 7 + 15 + 4);
        assert_eq!(path.get().join(&id).metadata().unwrap().len(), 7 + 15 + 4);
    }
}
