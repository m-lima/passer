#![deny(warnings, clippy::pedantic, clippy::all)]
#![warn(rust_2018_idioms)]

mod handler;
mod middleware;
mod options;
mod store;

#[cfg(not(feature = "host-frontend"))]
macro_rules! path {
    ($($path:literal)?) => {
        concat!("/", $($path)?)
    };
}

#[cfg(feature = "host-frontend")]
macro_rules! path {
    ($($path:literal)?) => {
        concat!("/api/", $($path)?)
    };
}

macro_rules! add_routes {
    ($route:ident, $options: ident, $cors:ident) => {
        add_routes!($route, $options);
        $route.options(path!()).to(|state| (state, ""));
    };
    ($route:ident, $options: ident) => {
        use gotham::router::builder::{DefineSingleRoute, DrawRoutes};
        #[cfg(feature = "host-frontend")]
        {
            log::info!("Serving front-end at {}", $options.web_path.0.display());
            $route
                .get("/*")
                .with_path_extractor::<gotham::handler::assets::FilePathExtractor>()
                .to_new_handler($crate::handler::Index::new(
                    $options.web_path.0,
                    $options.web_path.1.clone(),
                ));
            $route.get("/").to_file($options.web_path.1);
        }

        $route.post(path!()).to($crate::handler::post);
        $route
            .get(path!(":id"))
            .with_path_extractor::<IdExtractor>()
            .to($crate::handler::get);
    };
}

#[derive(serde::Deserialize, gotham_derive::StateData, gotham_derive::StaticResponseExtender)]
struct IdExtractor {
    id: String,
}

fn router(mut options: options::Options) -> gotham::router::Router {
    use gotham::middleware::state::StateMiddleware;
    use gotham::pipeline;
    use gotham::router::builder;

    let store: Box<dyn 'static + store::Store + Send> =
        if let Some(store_path) = options.store_path.take() {
            Box::new(store::InFile::new(store_path))
        } else {
            Box::new(store::InMemory::new())
        };

    if let Some(cors) = options.cors.take() {
        let pipeline = pipeline::new_pipeline()
            .add(middleware::Log)
            .add(StateMiddleware::new(middleware::Store::new(store)))
            .add(middleware::Cors::new(cors))
            .build();

        let (chain, pipelines) = pipeline::single::single_pipeline(pipeline);

        builder::build_router(chain, pipelines, |route| {
            add_routes!(route, options, cors);
        })
    } else {
        let pipeline = pipeline::new_pipeline()
            .add(middleware::Log)
            .add(StateMiddleware::new(middleware::Store::new(store)))
            .build();

        let (chain, pipelines) = pipeline::single::single_pipeline(pipeline);

        builder::build_router(chain, pipelines, |route| {
            add_routes!(route, options);
        })
    }
}

fn init_logger() {
    let config = simplelog::ConfigBuilder::new()
        .set_time_format_str("%Y-%m-%dT%H:%M:%SZ")
        .build();

    simplelog::TermLogger::init(
        simplelog::LevelFilter::Info,
        config,
        simplelog::TerminalMode::Mixed,
    )
    .expect("Could not initialize logger");
}

fn main() {
    let options = options::parse();
    init_logger();

    if options.threads > 0 {
        let threads = usize::from(options.threads);
        log::info!("Core threads set to {}", options.threads);
        gotham::start_with_num_threads(
            format!("0.0.0.0:{}", options.port),
            router(options),
            threads,
        );
    } else {
        log::info!("Core threads set to automatic");
        gotham::start(format!("0.0.0.0:{}", options.port), router(options));
    }
}

#[cfg(test)]
mod tests {
    use super::options;
    use super::router;
    use gotham::hyper;
    use gotham::test::TestServer;

    macro_rules! host_path {
        ($($path:literal)?) => {
            concat!("http://localhost", path!($($path)?))
        };
    }

    fn options() -> options::Options {
        options::Options {
            port: 0,
            threads: 0,
            cors: None,
            store_path: None,
            #[cfg(feature = "host-frontend")]
            web_path: ("res/test".into(), "res/test/index".into()),
        }
    }

    #[test]
    fn path() {
        {
            let path = path!();

            #[cfg(not(feature = "host-frontend"))]
            assert_eq!(path, "/");
            #[cfg(feature = "host-frontend")]
            assert_eq!(path, "/api/");
        }
        {
            let path = path!("foo/bar");

            #[cfg(not(feature = "host-frontend"))]
            assert_eq!(path, "/foo/bar");
            #[cfg(feature = "host-frontend")]
            assert_eq!(path, "/api/foo/bar");
        }
    }

    #[test]
    fn host_path() {
        {
            let path = host_path!();

            #[cfg(not(feature = "host-frontend"))]
            assert_eq!(path, "http://localhost/");
            #[cfg(feature = "host-frontend")]
            assert_eq!(path, "http://localhost/api/");
        }
        {
            let path = host_path!("foo/bar");

            #[cfg(not(feature = "host-frontend"))]
            assert_eq!(path, "http://localhost/foo/bar");
            #[cfg(feature = "host-frontend")]
            assert_eq!(path, "http://localhost/api/foo/bar");
        }
    }

    #[test]
    fn post_secret() {
        let test_server = TestServer::new(router(options())).unwrap();
        let response = test_server
            .client()
            .post(host_path!(), "foo", mime::TEXT_PLAIN)
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::CREATED);

        let body = response.read_body().unwrap();
        assert_eq!(body.len(), 43);
    }

    #[test]
    fn get_secret() {
        let test_server = TestServer::new(router(options())).unwrap();
        let response = test_server
            .client()
            .post(host_path!(), "foo", mime::TEXT_PLAIN)
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
        let test_server = TestServer::new(router(options())).unwrap();
        let response = test_server
            .client()
            .post(host_path!("my_key"), "foo", mime::TEXT_PLAIN)
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::METHOD_NOT_ALLOWED);

        let body = response.read_body().unwrap();
        assert!(body.is_empty());
    }

    #[test]
    fn cannot_put_empty_values() {
        let test_server = TestServer::new(router(options())).unwrap();
        let response = test_server
            .client()
            .post(host_path!(), "", mime::TEXT_PLAIN)
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::BAD_REQUEST);

        let body = response.read_body().unwrap();
        assert!(body.is_empty());
    }

    #[test]
    fn only_get_if_exists() {
        let test_server = TestServer::new(router(options())).unwrap();
        let response = test_server
            .client()
            .get(host_path!("foo"))
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::NOT_FOUND);

        let body = response.read_body().unwrap();
        assert!(body.is_empty());
    }

    #[test]
    fn no_cors() {
        let test_server = TestServer::new(router(options())).unwrap();
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

        let test_server = TestServer::new(router(options)).unwrap();
        let response = test_server
            .client()
            .get(host_path!("foo"))
            .perform()
            .unwrap();

        let cors = response
            .headers()
            .get(hyper::header::ACCESS_CONTROL_ALLOW_ORIGIN)
            .unwrap();

        assert_eq!(cors, "bar");
    }

    #[test]
    #[cfg(feature = "host-frontend")]
    fn index() {
        let test_server = TestServer::new(router(options())).unwrap();
        let response = test_server
            .client()
            .get("http://localhost")
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::OK);
        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"main_page\n");

        let response = test_server
            .client()
            .get("http://localhost/")
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::OK);
        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"main_page\n");
    }

    #[test]
    #[cfg(feature = "host-frontend")]
    fn assests() {
        let test_server = TestServer::new(router(options())).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/foo")
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::OK);
        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"bar\n");
    }

    #[test]
    #[cfg(feature = "host-frontend")]
    fn fallback_to_index() {
        let test_server = TestServer::new(router(options())).unwrap();
        let response = test_server
            .client()
            .get("http://localhost/bar")
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::OK);
        let body = response.read_body().unwrap();
        assert_eq!(&body[..], b"main_page\n");
    }

    #[test]
    #[cfg(feature = "host-frontend")]
    fn api_still_gets_served() {
        let test_server = TestServer::new(router(options())).unwrap();
        let response = test_server
            .client()
            .get(host_path!("foo"))
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::NOT_FOUND);
    }
}
