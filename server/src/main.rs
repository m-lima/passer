#![deny(warnings, clippy::pedantic, clippy::all)]
#![warn(rust_2018_idioms)]

mod options;
mod server;
mod store;

fn init_logger() {
    let config = simplelog::ConfigBuilder::new()
        .set_time_format_str("%Y-%m-%dT%H:%M:%SZ")
        .build();

    let color_choice = std::env::var("CLICOLOR_FORCE")
        .ok()
        .filter(|force| force != "0")
        .map(|_| simplelog::ColorChoice::Always)
        .or({
            std::env::var("CLICOLOR")
                .ok()
                .filter(|clicolor| clicolor == "0")
                .map(|_| simplelog::ColorChoice::Never)
        })
        .unwrap_or(simplelog::ColorChoice::Auto);

    simplelog::TermLogger::init(
        #[cfg(debug_assertions)]
        simplelog::LevelFilter::Debug,
        #[cfg(not(debug_assertions))]
        simplelog::LevelFilter::Info,
        config,
        simplelog::TerminalMode::Mixed,
        color_choice,
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
            server::route(options),
            threads,
        );
    } else {
        log::info!("Core threads set to automatic");
        gotham::start(format!("0.0.0.0:{}", options.port), server::route(options));
    }
}
