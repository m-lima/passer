use super::Error;

pub struct Store {
    secrets: std::collections::HashMap<String, Secret>,
}

impl Store {
    const MAX_SIZE: u64 = super::MAX_SECRET_SIZE * 10;

    pub fn new() -> Self {
        log::info!("Serving secrets from memory");
        Self {
            secrets: std::collections::HashMap::<_, _>::new(),
        }
    }

    fn size(&self) -> u64 {
        self.secrets
            .values()
            .map(|s| s.data.len())
            .fold(0, |a, c| a + (c as u64))
    }
}

impl super::Store for Store {
    fn refresh(&mut self) {
        self.secrets
            .retain(|_, secret| secret.expiry > std::time::SystemTime::now());
    }

    fn put(&mut self, expiry: std::time::SystemTime, data: Vec<u8>) -> Result<String, Error> {
        let size = data.len() as u64;

        if size > super::MAX_SECRET_SIZE {
            return Err(Error::TooLarge);
        }

        if self.size() + size > Self::MAX_SIZE {
            return Err(Error::StoreFull);
        }

        let id = loop {
            let id = super::new_id();
            if !self.secrets.contains_key(&id) {
                break id;
            }
        };

        self.secrets.insert(id.clone(), Secret { expiry, data });
        Ok(id)
    }

    fn get(&mut self, id: &str) -> Result<Vec<u8>, Error> {
        self.secrets
            .remove(id)
            .map(|s| s.data)
            .ok_or(Error::SecretNotFound)
    }
}

struct Secret {
    expiry: std::time::SystemTime,
    data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::super::Store as Trait;
    use super::Store;

    #[test]
    fn put() {
        let mut store = Store::new();
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
        assert_eq!(store.secrets.len(), 1);
    }

    #[test]
    fn get() {
        let mut store = Store::new();
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

        assert!(store.secrets.is_empty());
        assert_eq!(&result[..], b"test");
    }

    #[test]
    fn refresh() {
        let mut store = Store::new();
        let data: Vec<u8> = b"test"[..].into();
        store
            .put(
                std::time::SystemTime::now()
                    .checked_add(std::time::Duration::from_millis(50))
                    .unwrap(),
                data,
            )
            .unwrap();

        assert_eq!(store.secrets.len(), 1);
        std::thread::sleep(std::time::Duration::from_millis(200));

        store.refresh();
        assert!(store.secrets.is_empty());
    }

    #[test]
    fn size() {
        let mut store = Store::new();
        let data: Vec<u8> = b"test"[..].into();
        let len = data.len() as u64;
        store
            .put(
                std::time::SystemTime::now()
                    .checked_add(std::time::Duration::from_secs(1))
                    .unwrap(),
                data,
            )
            .unwrap();

        assert_eq!(store.size(), len);
    }
}
