use gotham::hyper;

use super::middleware;
use super::IdExtractor;

// #[cfg(feature = "host-frontend")]
#[derive(Clone)]
pub struct Index(
    gotham::handler::assets::DirHandler,
    gotham::handler::assets::FileHandler,
);

#[cfg(feature = "host-frontend")]
impl Index {
    pub fn new(root: std::path::PathBuf, index: std::path::PathBuf) -> Self {
        use gotham::handler::assets;
        Self(
            assets::DirHandler::new(assets::FileOptions::from(root)),
            assets::FileHandler::new(assets::FileOptions::from(index)),
        )
    }
}

#[cfg(feature = "host-frontend")]
impl gotham::handler::NewHandler for Index {
    type Instance = Self;

    fn new_handler(&self) -> gotham::anyhow::Result<Self::Instance> {
        Ok(self.clone())
    }
}

#[cfg(feature = "host-frontend")]
impl gotham::handler::Handler for Index {
    fn handle(
        self,
        state: gotham::state::State,
    ) -> std::pin::Pin<Box<gotham::handler::HandlerFuture>> {
        Box::pin(async {
            // Allowed because this is third-party code being flagged
            #[allow(clippy::used_underscore_binding)]
            match self.0.handle(state).await {
                Ok(response) => Ok(response),
                Err((state, _)) => self.1.handle(state).await,
            }
        })
    }
}

pub fn get(
    mut state: gotham::state::State,
) -> (gotham::state::State, hyper::Response<hyper::Body>) {
    use gotham::handler::IntoResponse;
    use gotham::state::FromState;

    let id = { IdExtractor::take_from(&mut state).id };
    let store = middleware::Store::borrow_mut_from(&mut state);

    let response = store
        .get(&id)
        .map_or_else(|e| e.into_response(&state), |r| r.into_response(&state));
    (state, response)
}

pub fn post(mut state: gotham::state::State) -> std::pin::Pin<Box<gotham::handler::HandlerFuture>> {
    Box::pin(async {
        use gotham::handler::IntoResponse;
        use gotham::state::FromState;
        use hyper::{body, Body};

        // Allowed because this is third-party code being flagged
        #[allow(clippy::used_underscore_binding)]
        match body::to_bytes(Body::take_from(&mut state))
            .await
            .map(|b| b.to_vec())
            .map_err(|e| e.into())
            .and_then(|data| {
                let store = middleware::Store::borrow_mut_from(&mut state);
                store
                    .put(
                        data,
                        std::time::SystemTime::now()
                            .checked_add(std::time::Duration::from_secs(24 * 60 * 60))
                            .unwrap(),
                    )
                    .map(|key| {
                        let mut response = key.into_response(&state);
                        *response.status_mut() = hyper::StatusCode::CREATED;
                        response
                    })
            }) {
            Ok(r) => Ok((state, r)),
            Err(e) => Err((state, e)),
        }
    })
}
