use crate::store;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("nothing to insert")]
    NothingToInsert,
    #[error("failed to acquire store")]
    FailedToAcquireStore,
    #[error("{0}")]
    Store(store::Error),
    #[error("{0}")]
    Unknown(String),
}

impl Error {
    fn status_code(&self) -> gotham::hyper::StatusCode {
        use super::store::Error as StoreError;
        use gotham::hyper::StatusCode;

        match self {
            Error::NothingToInsert => StatusCode::BAD_REQUEST,
            Error::Store(StoreError::TooLarge) => StatusCode::PAYLOAD_TOO_LARGE,
            Error::Store(StoreError::StoreFull) => StatusCode::CONFLICT,
            Error::Store(StoreError::SecretNotFound) => StatusCode::NOT_FOUND,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn into_handler_error(self) -> gotham::handler::HandlerError {
        let status = self.status_code();
        gotham::handler::HandlerError::from(self).with_status(status)
    }
}

impl From<store::Error> for Error {
    fn from(e: store::Error) -> Self {
        Self::Store(e)
    }
}
