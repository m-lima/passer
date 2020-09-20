use super::handler;
use super::middleware;
use super::options::Options;
use super::store;

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

pub fn route(options: Options) -> gotham::router::Router {
    use gotham::pipeline;
    use gotham::router::builder;

    #[cfg(feature = "host-frontend")]
    let web_path = options.web_path;

    let pipeline = pipeline::new_pipeline()
        .add(middleware::Log)
        .add(options.store_path.map_or_else(
            || middleware::Store::new(store::in_memory()),
            |path| middleware::Store::new(store::in_file(path)),
        ))
        .build();

    let (chain, pipelines) = pipeline::single::single_pipeline(pipeline);

    builder::build_router(chain, pipelines, |route| {
        use gotham::router::builder::{DefineSingleRoute, DrawRoutes};

        #[cfg(feature = "host-frontend")]
        {
            log::info!("Serving front-end at {}", web_path.0.display());
            route
                .get("/*")
                .with_path_extractor::<gotham::handler::assets::FilePathExtractor>()
                .to_new_handler(handler::Index::new(web_path.0, web_path.1.clone()));
            route.get("/").to_file(web_path.1);
        }

        route
            .post(path!())
            .with_query_string_extractor::<handler::TtlExtractor>()
            .to(handler::post);
        route
            .get(path!(":id"))
            .with_path_extractor::<handler::IdExtractor>()
            .to(handler::get);
    })
}

#[cfg(test)]
mod tests {
    use gotham::hyper;
    use gotham::test::TestServer;

    use super::super::options;
    use super::route;

    macro_rules! host_path {
        ($($path:literal)?) => {
            concat!("http://localhost", path!($($path)?))
        };
    }

    fn options() -> options::Options {
        options::Options {
            port: 0,
            threads: 0,
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
    #[cfg(feature = "host-frontend")]
    fn index() {
        let test_server = TestServer::new(route(options())).unwrap();
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
    fn assets() {
        let test_server = TestServer::new(route(options())).unwrap();
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
        let test_server = TestServer::new(route(options())).unwrap();
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
        let test_server = TestServer::new(route(options())).unwrap();
        let response = test_server
            .client()
            .get(host_path!("0___________________foo___________________0"))
            .perform()
            .unwrap();

        assert_eq!(response.status(), hyper::StatusCode::NOT_FOUND);
    }
}
