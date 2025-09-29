use std::process;

use clap::Parser;

mod cli;
mod cmd;
mod docker;
mod net;
mod protocol;
mod result;

fn main() {
    let opts = match cli::Opts::try_parse() {
        Ok(o) => o,
        Err(e) => {
            docker::error!("Failed to parse args: {}", e);
            process::exit(1)
        }
    };
    cli::run(opts)
}
