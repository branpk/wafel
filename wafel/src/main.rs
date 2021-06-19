use log::LevelFilter;

// TODO
// #![warn(
//     missing_docs,
//     missing_debug_implementations,
//     rust_2018_idioms,
//     unreachable_pub
// )]

fn main() {
    env_logger::builder().filter_level(LevelFilter::Info).init(); // TODO: Replace with log file

    log::info!("running");
}

async fn run() {}
