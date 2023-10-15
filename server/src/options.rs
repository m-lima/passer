pub fn parse() -> Options {
    <Options as clap::Parser>::parse()
}

#[derive(clap::Parser)]
pub struct Options {
    /// Selects the port to serve on
    #[clap(short, long, default_value = "80")]
    pub port: u16,

    /// Selects the number of threads to use. Zero for automatic
    #[clap(short, long, default_value = "0")]
    pub threads: u8,

    /// Sets the 'allow-origin' header
    #[clap(short, long, value_parser = to_cors)]
    pub cors: Option<gotham::hyper::header::HeaderValue>,

    /// Sets storage location
    ///
    /// Will store secrets in memory if no path is provided
    #[clap(short, long, value_parser = clap::builder::TypedValueParser::try_map(clap::builder::PathBufValueParser::new(), to_dir_path))]
    pub store_path: Option<std::path::PathBuf>,

    /// The directory of the front-end content
    ///
    /// If set, the front-end will be served on the root path "/"
    /// and the api will be nested under "/api"
    #[clap(short, long, value_parser = clap::builder::TypedValueParser::try_map(clap::builder::PathBufValueParser::new(), to_index_root))]
    pub web_path: Option<(std::path::PathBuf, std::path::PathBuf)>,
}

fn to_cors(
    value: &str,
) -> Result<gotham::hyper::header::HeaderValue, gotham::hyper::header::InvalidHeaderValue> {
    gotham::hyper::header::HeaderValue::from_str(value)
}

fn to_dir_path(path: std::path::PathBuf) -> Result<std::path::PathBuf, &'static str> {
    if !path.is_dir() {
        return Err("path is not a directory");
    }

    Ok(path)
}

fn to_index_root(
    path: std::path::PathBuf,
) -> Result<(std::path::PathBuf, std::path::PathBuf), &'static str> {
    if !path.is_dir() {
        return Err("path is not a directory");
    }

    let index = path.join("index.html");
    if !index.exists() {
        return Err("path does not contain an index.html");
    }

    Ok((path, index))
}
