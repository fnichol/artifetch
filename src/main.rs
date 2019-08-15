use artifetch::app::{self, Error};
use std::process;

fn main() {
    if let Err(err) = try_main() {
        eprintln!("error: {}", err);
        process::exit(1);
    }
}

fn try_main() -> Result<(), Error> {
    std::env::set_var(
        "RUST_LOG",
        "actix_server=info,actix_web=info,artifetch=info",
    );
    env_logger::init();

    app::run(app::config()?)
}
