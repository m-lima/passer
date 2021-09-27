use crate::store;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("nothing to insert")]
    NothingToInsert,
    #[error("failed to acquire store")]
    FailedToAcquireStore,
    #[error("content length missing")]
    ContentLengthMissing,
    #[error("payload too large")]
    PayloadTooLarge,
    #[error("read timeout")]
    ReadTimeout,
    #[error("{0}")]
    Hyper(gotham::hyper::Error),
    #[error("{0}")]
    Store(store::Error),
}

impl Error {
    fn status_code(&self) -> gotham::hyper::StatusCode {
        use super::store::Error as StoreError;
        use gotham::hyper::StatusCode;

        match self {
            Error::Store(StoreError::SecretNotFound) => StatusCode::NOT_FOUND,
            Error::Store(StoreError::InvalidId(_)) => StatusCode::BAD_REQUEST,
            Error::ContentLengthMissing => StatusCode::LENGTH_REQUIRED,
            Error::NothingToInsert => StatusCode::UNPROCESSABLE_ENTITY,
            Error::ReadTimeout => StatusCode::REQUEST_TIMEOUT,
            Error::Store(StoreError::StoreFull) => StatusCode::CONFLICT,
            Error::Store(StoreError::TooLarge) | Error::PayloadTooLarge => {
                StatusCode::PAYLOAD_TOO_LARGE
            }
            Error::Hyper(_)
            | Error::FailedToAcquireStore
            | Error::Store(StoreError::Generic(_)) => StatusCode::INTERNAL_SERVER_ERROR,
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
