use clap::Clap;
use gotham::hyper;

pub fn parse() -> Options {
    Options::parse()
}

#[derive(Clap)]
pub struct Options {
    /// Selects the port to serve on
    #[clap(short, long, default_value = "80")]
    pub port: u16,

    /// Sets the 'allow-origin' header
    #[clap(short, long, parse(try_from_str = to_cors))]
    pub cors: Option<hyper::header::HeaderValue>,

    /// Selects the number of threads to use. Zero for automatic
    #[clap(short, long, default_value = "0")]
    pub threads: u8,

    /// The directory of the front-end content
    #[cfg(feature = "host-frontend")]
    #[clap(short, long, parse(try_from_str = to_path))]
    pub web_path: std::path::PathBuf,
}

fn to_cors(value: &str) -> Result<hyper::header::HeaderValue, hyper::header::InvalidHeaderValue> {
    hyper::header::HeaderValue::from_str(value)
}

#[cfg(feature = "host-frontend")]
fn to_path(value: &str) -> Result<std::path::PathBuf, &'static str> {
    let path = std::path::PathBuf::from(value);

    if path.is_dir() {
        Ok(path)
    } else {
        Err("path is not a directory")
    }
}
