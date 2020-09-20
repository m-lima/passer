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
