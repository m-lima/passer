use clap::Clap;

pub fn parse() -> Options {
    Options::parse()
}

#[derive(Clap)]
pub struct Options {
    /// Selects the port to serve on
    #[clap(short, long, default_value = "80")]
    pub port: u16,

    /// Selects the number of threads to use. Zero for automatic
    #[clap(short, long, default_value = "0")]
    pub threads: u8,

    /// Sets storage location
    #[clap(short, long, parse(try_from_str = to_dir_path))]
    pub store_path: Option<std::path::PathBuf>,

    /// The directory of the front-end content
    #[cfg(feature = "host-frontend")]
    #[clap(short, long, parse(try_from_str = to_index_root))]
    pub web_path: (std::path::PathBuf, std::path::PathBuf),
}

fn to_dir_path(value: &str) -> Result<std::path::PathBuf, &'static str> {
    let path = std::path::PathBuf::from(value);
    if !path.is_dir() {
        return Err("path is not a directory");
    }

    Ok(path)
}

#[cfg(feature = "host-frontend")]
fn to_index_root(value: &str) -> Result<(std::path::PathBuf, std::path::PathBuf), &'static str> {
    let path = std::path::PathBuf::from(value);
    if !path.is_dir() {
        return Err("path is not a directory");
    }

    let index = path.join("index.html");
    if !index.exists() {
        return Err("path does not contain an index.html");
    }

    Ok((path, index))
}
