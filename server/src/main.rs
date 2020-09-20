#![deny(warnings, clippy::pedantic, clippy::all)]
#![warn(rust_2018_idioms)]

mod handler;
mod middleware;
mod options;
mod router;
mod store;

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
            router::route(options),
            threads,
        );
    } else {
        log::info!("Core threads set to automatic");
        gotham::start(format!("0.0.0.0:{}", options.port), router::route(options));
    }
}
