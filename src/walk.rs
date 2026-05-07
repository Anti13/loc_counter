use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;

use ignore::{WalkBuilder, WalkState};

use crate::count::count_file;
use crate::lang::{self, Language};

#[derive(Debug, Default, Clone, Copy)]
pub struct LanguageStats {
    pub files: u64,
    pub lines: u64,
}

#[derive(Debug, Default)]
pub struct ScanResult {
    pub per_language: HashMap<Language, LanguageStats>,
    pub skipped_binary: u64,
    pub read_errors: u64,
}

#[derive(Debug, Clone, Copy)]
pub struct WalkOptions {
    pub respect_ignore: bool,
    pub include_hidden: bool,
}

impl Default for WalkOptions {
    fn default() -> Self {
        Self {
            respect_ignore: true,
            include_hidden: false,
        }
    }
}

/// Walks `root` in parallel and returns aggregated counts.
///
/// Per-thread accumulators are owned by the visitor closures `ignore` hands to
/// each worker. A `Drop` guard inside the closure flushes the local result into
/// a shared `Mutex<Vec<_>>` when the thread tears the closure down — that's the
/// only contention point, and it happens once per worker.
pub fn scan(root: &Path, options: WalkOptions) -> ScanResult {
    let collected: Mutex<Vec<ScanResult>> = Mutex::new(Vec::new());

    let walker = WalkBuilder::new(root)
        .standard_filters(options.respect_ignore)
        .hidden(!options.include_hidden)
        .follow_links(false)
        .threads(0)
        .build_parallel();

    walker.run(|| {
        let mut guard = LocalGuard {
            local: ScanResult::default(),
            sink: &collected,
        };
        Box::new(move |entry| {
            visit(&mut guard.local, entry);
            WalkState::Continue
        })
    });

    merge_results(collected.into_inner().unwrap())
}

struct LocalGuard<'a> {
    local: ScanResult,
    sink: &'a Mutex<Vec<ScanResult>>,
}

impl Drop for LocalGuard<'_> {
    fn drop(&mut self) {
        let taken = std::mem::take(&mut self.local);
        if let Ok(mut sink) = self.sink.lock() {
            sink.push(taken);
        }
    }
}

fn visit(local: &mut ScanResult, entry: Result<ignore::DirEntry, ignore::Error>) {
    let dir_entry = match entry {
        Ok(dir_entry) => dir_entry,
        Err(err) => {
            local.read_errors += 1;
            eprintln!("warning: walk error: {err}");
            return;
        }
    };

    if !dir_entry
        .file_type()
        .is_some_and(|file_type| file_type.is_file())
    {
        return;
    }
    let Some(language) = lang::from_path(dir_entry.path()) else {
        return;
    };

    match count_file(dir_entry.path()) {
        Ok(counts) if counts.binary => {
            local.skipped_binary += 1;
        }
        Ok(counts) => {
            let stats = local.per_language.entry(language).or_default();
            stats.files += 1;
            stats.lines += counts.lines;
        }
        Err(err) => {
            local.read_errors += 1;
            eprintln!(
                "warning: could not read {}: {}",
                dir_entry.path().display(),
                err
            );
        }
    }
}

fn merge_results(parts: Vec<ScanResult>) -> ScanResult {
    let mut total = ScanResult::default();
    for part in parts {
        total.skipped_binary += part.skipped_binary;
        total.read_errors += part.read_errors;
        for (language, stats) in part.per_language {
            let entry = total.per_language.entry(language).or_default();
            entry.files += stats.files;
            entry.lines += stats.lines;
        }
    }
    total
}
