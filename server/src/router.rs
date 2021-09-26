use super::handler;
use super::middleware;
use super::options::Options;
use super::store;

macro_rules! path {
    ($web_path: ident $(, $path:literal)?) => {
        if $web_path.is_some() {
            concat!("/api/", $($path)?)
        } else {
            concat!("/", $($path)?)
        };
    };
}

macro_rules! add_routes {
    ($route:ident, $web_path: ident, cors) => {{
        use gotham::router::builder::{DefineSingleRoute, DrawRoutes};
        $route.options(path!($web_path)).to(|state| (state, ""));
        add_routes!($route, $web_path);
    }};
    ($route:ident, $web_path: ident) => {{
        use gotham::router::builder::{DefineSingleRoute, DrawRoutes};

        $route
            .post(path!($web_path))
            .with_query_string_extractor::<handler::TtlExtractor>()
            .to(handler::post);
        $route
            .get(path!($web_path, ":id"))
            .with_path_extractor::<handler::IdExtractor>()
            .to(handler::get);

        if let Some(web_path) = $web_path {
            log::info!("Serving front-end at {}", web_path.0.display());
            $route
                .get("/*")
                .with_path_extractor::<gotham::handler::assets::FilePathExtractor>()
                .to_new_handler(handler::Index::new(web_path.0, web_path.1.clone()));
            $route.get("/").to_file(web_path.1);
        }
    }};
}

// Allowed because you can't create closures that share the same captures
#[allow(clippy::option_if_let_else)]
pub fn route(options: Options) -> gotham::router::Router {
    use gotham::pipeline;
    use gotham::router::builder;

    let web_path = options.web_path;

    let store = options.store_path.map_or_else(
        || middleware::Store::new(store::in_memory()),
        |path| middleware::Store::new(store::in_file(path)),
    );

    if let Some(cors) = options.cors {
        let pipeline = pipeline::new_pipeline()
            .add(middleware::Log)
            .add(store)
            .add(middleware::Cors::new(cors))
            .build();

        let (chain, pipelines) = pipeline::single::single_pipeline(pipeline);

        builder::build_router(chain, pipelines, |route| {
            add_routes!(route, web_path, cors);
        })
    } else {
        let pipeline = pipeline::new_pipeline()
            .add(middleware::Log)
            .add(store)
            .build();

        let (chain, pipelines) = pipeline::single::single_pipeline(pipeline);

        builder::build_router(chain, pipelines, |route| {
            add_routes!(route, web_path);
        })
    }
}

#[cfg(test)]
mod tests {
    use gotham::hyper;
    use gotham::test::TestServer;

    use super::super::options;
    use super::route;

    macro_rules! host_path {
        ($($path:literal)?) => {
            concat!("http://localhost/", $($path)?)
        };
    }

    fn options() -> options::Options {
        options::Options {
            port: 0,
            threads: 0,
            cors: None,
            store_path: None,
            web_path: None,
        }
    }

    fn options_with_path() -> options::Options {
        options::Options {
            port: 0,
            threads: 0,
            cors: None,
            store_path: None,
            web_path: Some(("res/test".into(), "res/test/index".into())),
        }
    }

    #[test]
    fn path() {
        let some = Some(());
        let none: Option<()> = None;
        assert_eq!(path!(none), "/");
        assert_eq!(path!(none, "foo/bar"), "/foo/bar");
        assert_eq!(path!(some), "/api/");
        assert_eq!(path!(some, "foo/bar"), "/api/foo/bar");
    }

    #[test]
    fn host_path() {
        assert_eq!(host_path!(), "http://localhost/");
        assert_eq!(host_path!("foo/bar"), "http://localhost/foo/bar");
    }

    #[test]
    fn post_secret() {
        let test_server = TestServer::new(route(options())).unwrap();
        let response = test_server
            .client()
            .post(concat!(host_path!(), "?ttl=1m"), "foo", mime::TEXT_PLAIN)
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::CREATED);

        let body = response.read_body().unwrap();
        assert_eq!(body.len(), 43);
    }

    #[test]
    fn get_secret() {
        let test_server = TestServer::new(route(options())).unwrap();
        let response = test_server
            .client()
            .post(concat!(host_path!(), "?ttl=1m"), "foo", mime::TEXT_PLAIN)
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::CREATED);
        let key = response.read_body().unwrap();

        let response = test_server
            .client()
            .get(format!(
                concat!(host_path!(), "{}"),
                key.into_iter().map(|c| c as char).collect::<String>()
            ))
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::OK);

        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"foo");
    }

    #[test]
    fn cannot_choose_key_to_put() {
        let test_server = TestServer::new(route(options())).unwrap();
        let response = test_server
            .client()
            .post(
                concat!(host_path!("my_key"), "?ttl=1m"),
                "foo",
                mime::TEXT_PLAIN,
            )
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::METHOD_NOT_ALLOWED);

        let body = response.read_body().unwrap();
        assert!(body.is_empty());
    }

    #[test]
    fn cannot_put_empty_values() {
        let test_server = TestServer::new(route(options())).unwrap();
        let response = test_server
            .client()
            .post(concat!(host_path!(), "?ttl=1m"), "", mime::TEXT_PLAIN)
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::BAD_REQUEST);

        let body = response.read_body().unwrap();
        assert!(body.is_empty());
    }

    #[test]
    fn cannot_omit_ttl() {
        let test_server = TestServer::new(route(options())).unwrap();
        let response = test_server
            .client()
            .post(host_path!(), "foo", mime::TEXT_PLAIN)
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::BAD_REQUEST);

        let body = response.read_body().unwrap();
        assert!(body.is_empty());
    }

    #[test]
    fn cannot_use_malformed_ttl() {
        let test_server = TestServer::new(route(options())).unwrap();
        let response = test_server
            .client()
            .post(concat!(host_path!(), "?ttl=1"), "", mime::TEXT_PLAIN)
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::BAD_REQUEST);

        let body = response.read_body().unwrap();
        assert!(body.is_empty());
    }

    #[test]
    fn secrets_expire() {
        let test_server = TestServer::new(route(options())).unwrap();
        let response = test_server
            .client()
            .post(concat!(host_path!(), "?ttl=0m"), "foo", mime::TEXT_PLAIN)
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::CREATED);
        let key = response.read_body().unwrap();

        let response = test_server
            .client()
            .get(format!(
                concat!(host_path!(), "{}"),
                key.into_iter().map(|c| c as char).collect::<String>()
            ))
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::NOT_FOUND);

        let body = response.read_body().unwrap();
        assert!(body.is_empty());
    }

    #[test]
    fn only_get_if_exists() {
        let test_server = TestServer::new(route(options())).unwrap();
        let response = test_server
            .client()
            .get(host_path!("0___________________foo___________________0"))
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::NOT_FOUND);

        let body = response.read_body().unwrap();
        assert!(body.is_empty());
    }

    #[test]
    fn reject_bad_ids() {
        let test_server = TestServer::new(route(options())).unwrap();
        let response = test_server
            .client()
            .get(host_path!("foo"))
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::BAD_REQUEST);

        let body = response.read_body().unwrap();
        assert!(body.is_empty());
    }

    #[test]
    fn no_cors() {
        let test_server = TestServer::new(route(options())).unwrap();
        let response = test_server
            .client()
            .get(host_path!("foo"))
            .perform()
            .unwrap();

        let cors = response
            .headers()
            .get(hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN);

        assert!(cors.is_none());
    }

    #[test]
    fn with_cors() {
        let mut options = options();
        options.cors = Some(hyper::header::HeaderValue::from_static("bar"));

        let test_server = TestServer::new(route(options)).unwrap();
        let response = test_server
            .client()
            .get(host_path!("0___________________foo___________________0"))
            .perform()
            .unwrap();

        let cors = response
            .headers()
            .get(hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .unwrap();

        assert_eq!(cors, "bar");
    }

    #[test]
    fn preflight() {
        let mut options = options();
        options.cors = Some(hyper::header::HeaderValue::from_static("bar"));

        let test_server = TestServer::new(route(options)).unwrap();
        let response = test_server
            .client()
            .options(host_path!())
            .perform()
            .unwrap();

        let cors = response
            .headers()
            .get(hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .unwrap();

        assert_eq!(cors, "bar");
    }

    #[test]
    fn index() {
        let test_server = TestServer::new(route(options_with_path())).unwrap();
        let response = test_server
            .client()
            .get("http://localhost")
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::OK);
        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"main_page\n");

        let response = test_server.client().get(host_path!()).perform().unwrap();

        assert_eq!(response.status(), hyper::StatusCode::OK);
        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"main_page\n");
    }

    #[test]
    fn assets() {
        let test_server = TestServer::new(route(options_with_path())).unwrap();
        let response = test_server
            .client()
            .get(host_path!("foo"))
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::OK);
        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"bar\n");
    }

    #[test]
    fn fallback_to_index() {
        let test_server = TestServer::new(route(options_with_path())).unwrap();
        let response = test_server
            .client()
            .get(host_path!("bar"))
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::OK);
        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"main_page\n");
    }

    #[test]
    fn api_still_gets_served() {
        let test_server = TestServer::new(route(options_with_path())).unwrap();
        let response = test_server
            .client()
            .get(host_path!(
                "api/0___________________foo___________________0"
            ))
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::NOT_FOUND);
    }
}
