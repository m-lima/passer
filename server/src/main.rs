use gotham_derive::{StateData, StaticResponseExtender};
use serde::Deserialize;

#[derive(Deserialize, StateData, StaticResponseExtender)]
struct IdExtractor {
    id: String,
}

// #[derive(Clone)]
// struct Store {
//     secrets
// }

fn post_handler(
    mut state: gotham::state::State,
) -> std::pin::Pin<Box<gotham::handler::HandlerFuture>> {
    use futures::future::{self, FutureExt};

    async {
        use gotham::handler::{IntoHandlerError, IntoResponse};
        use gotham::hyper::{body, Body};
        use gotham::state::FromState;

        let response = body::to_bytes(Body::take_from(&mut state))
            .await
            .map_err(IntoHandlerError::into_handler_error)
            .into_response(&state);
        (state, response)
    }
    .then(future::ok)
    .boxed()
}

fn router() -> gotham::router::Router {
    gotham::router::builder::build_simple_router(|route| {
        use gotham::router::builder::*;
        route
            .get_or_head("/:id")
            .with_path_extractor::<IdExtractor>()
            .to(|mut state| {
                use gotham::state::FromState;
                let id = { IdExtractor::take_from(&mut state).id };
                (state, format!("Getting {}", id))
            });
        route.post("/").to(post_handler)
    })
}

fn main() {
    gotham::start_with_num_threads("0.0.0.0:3030", router(), 1);
    // gotham::start("0.0.0.0:3030", router());
}
