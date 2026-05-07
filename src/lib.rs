pub mod cli;
pub mod count;
pub mod lang;
pub mod report;
pub mod walk;

use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};

pub fn run(path: &Path, no_ignore: bool, hidden: bool, out: impl Write) -> Result<()> {
    let path = path
        .canonicalize()
        .with_context(|| format!("could not access path: {}", path.display()))?;

    let opts = walk::WalkOptions {
        respect_ignore: !no_ignore,
        include_hidden: hidden,
    };
    let result = walk::scan(&path, opts);
    report::print(&result, out).context("failed writing report")?;
    Ok(())
}
