use artifetch::app::{self, Error};
use log::{debug, error};
use std::process;
use structopt::StructOpt;

mod cli;

fn main() {
    cli::util::init_logger();

    if let Err(err) = try_main() {
        error!("{}", err);
        process::exit(1);
    }
}

fn try_main() -> Result<(), Error> {
    let args = cli::Args::from_args();
    debug!("parsed cli arguments; args={:?}", args);

    app::run(app::config(args.config_path())?)
}
