use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "loc_counter",
    version,
    about = "Count lines of code per language across a project tree."
)]
pub struct Args {
    /// Path to the project folder to scan.
    pub path: PathBuf,

    /// Include files normally ignored by .gitignore / .ignore.
    #[arg(long)]
    pub no_ignore: bool,

    /// Include hidden files and directories.
    #[arg(long)]
    pub hidden: bool,
}
