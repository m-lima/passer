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
        use gotham::handler::IntoHandlerError;
        use gotham::state::FromState;
        match gotham::hyper::body::to_bytes(gotham::hyper::Body::take_from(&mut state)).await {
            Ok(body) => {
                let response = gotham::handler::IntoResponse::into_response(body, &state);
                Ok((state, response))
            }
            Err(e) => Err((state, e.into_handler_error())),
        }
    }
    .then(|r| match r {
        Ok(r) => future::ok(r),
        Err(e) => future::err(e),
    })
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
