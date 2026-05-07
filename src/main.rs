use std::io;
use std::process::ExitCode;

use clap::Parser;

use loc_counter::cli::Args;

fn main() -> ExitCode {
    let args = Args::parse();
    match loc_counter::run(&args.path, args.no_ignore, args.hidden, io::stdout().lock()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err:#}");
            ExitCode::FAILURE
        }
    }
}
