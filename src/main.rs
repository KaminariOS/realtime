use log::LevelFilter;
use realtime::run;

fn main() {
    env_logger::Builder::new()
        .filter(Some("realtime"), LevelFilter::Info)
        .init();
    pollster::block_on(run());
}
