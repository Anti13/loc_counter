# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

`loc_counter` is a Rust CLI that recursively scans a project directory and reports lines of code per programming language plus a total. Inspired by `tokei`/`cloc` but intentionally minimal — no comment/blank-line breakdown, no JSON output, no per-file stats.

## Commands

- Build: `cargo build` (release: `cargo build --release`)
- Run: `cargo run -- <path-to-scan>` (e.g. `cargo run -- .`)
- Test: `cargo test`
- Single test: `cargo test <test_name>` (substring match) or `cargo test --test integration` for the integration file
- Lint: `cargo clippy --all-targets -- -D warnings`
- Format: `cargo fmt` (check-only: `cargo fmt -- --check`)

CLI flags:
- positional `<path>` (required) — folder to scan
- `--no-ignore` — include files normally excluded by `.gitignore` / `.ignore`
- `--hidden` — include hidden files and directories

## Architecture

Pipeline of small modules under `src/`. Each does one thing:

1. `cli.rs` — `clap` derive. Just the arg surface; no logic.
2. `lang.rs` — extension → `Language` enum. Adding a language means adding a row to `from_path` and a variant. Unknown extensions return `None` and the file is skipped (deliberate; keeps output focused).
3. `count.rs` — per-file line counting. Adaptive: files ≤ 64 KB go through `read_to_end` (single syscall, fewer allocations); larger files stream through a 64 KB `BufReader`. Newlines are counted with `bytecount::count` (SIMD popcount). The first ~8 KB of every file is scanned with `memchr::memchr` for null bytes — if found, the file is treated as binary and skipped (Git's heuristic).
4. `walk.rs` — drives `ignore::WalkBuilder::build_parallel()`. Each worker thread holds a thread-local `ScanResult`. A `Drop` guard flushes that local into a shared `Mutex<Vec<ScanResult>>` exactly once per worker (when its visitor closure is dropped at thread shutdown). The shared mutex is contended only N times where N = thread count. After the walk, all locals are merged into one `ScanResult`.
5. `report.rs` — sorts by language name (`BTreeMap`), prints an aligned table with thousands-separated counts and a `Total` row. Footer reports skipped binary files and per-file read errors when nonzero.

`lib.rs` exposes `run(path, no_ignore, hidden, out)` so integration tests can drive the pipeline directly without spawning the binary. `main.rs` is just argument parsing → `run()`.

## Robustness invariants — do not break these

- **Symlinks are not followed.** Eliminates loops and "escape the project root" bugs in v1. If you add `--follow-links` later, `ignore` already does loop detection — wire it through, don't reinvent it.
- **Per-file errors never abort the scan.** Permission denied, mid-read I/O failure, file deleted between walk and open: log to stderr, increment `read_errors`, continue. Only top-level errors (invalid root) return non-zero.
- **Memory is bounded.** No mmap. The small-file path is gated on `metadata().len()`; the large-file path streams. Don't introduce a "read everything into memory" path without that gate.
- **No `unsafe`.** If you reach for it, document why in the PR.
- **Output ordering is deterministic.** `BTreeMap` by language name. Tests rely on this.

## Performance notes

- The walker does work-stealing across logical CPUs (`.threads(0)` lets `ignore` pick).
- The hot path has zero locking. Per-thread accumulation, single merge at the end.
- `bytecount` is the right primitive for "count occurrences of byte X." Don't swap it back to `memchr_iter().count()`; that iterates positions and is slower. Keep `memchr` for the null-byte presence check.
- The 64 KB threshold for slurp-vs-stream is set to match the BufReader capacity. If you tune one, tune both.

## Adding a new language

Edit `src/lang.rs`: add the variant to the `Language` enum, a row in the extension match in `from_path`, and an entry in `name()`. `walk.rs` and `count.rs` are language-agnostic; they don't need to change.

## Encoding limitations

We count `\n` bytes. That's encoding-invariant for UTF-8, Latin-1, Windows-1252, and any other ASCII-superset. UTF-16 source files would undercount (one `\n` per two bytes); they're vanishingly rare in real source trees, so this is documented but not handled.
