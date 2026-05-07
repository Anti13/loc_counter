# loc_counter

A small, fast Rust CLI that counts lines of code per programming language across a project tree.

## Install / build

```sh
cargo build --release
# binary at target/release/loc_counter
```

## Usage

```sh
loc_counter <path>
loc_counter . --no-ignore --hidden
```

Flags:

| Flag | Meaning |
|---|---|
| `<path>` | Directory to scan (required). |
| `--no-ignore` | Include files normally excluded by `.gitignore` / `.ignore`. |
| `--hidden` | Include hidden files and directories. |

Symlinks are not followed.

## Example output

```
Language       Files      Lines
-------------  ------  ---------
C                  12      3,420
C++                 8      1,205
Rust               34     12,887
-------------  ------  ---------
Total              54     17,512
```

## How it works

- **Walking**: parallel directory walk via the `ignore` crate (the same walker used by ripgrep, fd, tokei). Respects `.gitignore`, skips `.git/`, handles symlink-loop detection.
- **Counting**: small files (≤ 64 KB) are slurped with one `read_to_end`; larger files stream through a 64 KB `BufReader`. Newlines are counted with `bytecount` (SIMD popcount). Binary files are detected via a null-byte check on the first 8 KB and skipped.
- **Aggregation**: each worker thread keeps a thread-local accumulator. They flush into a shared vec exactly once (at thread shutdown). Final merge produces the report.

No `unsafe`, no mmap, no shelling out.

## Supported languages (v1)

C, C++, C#, Go, Java, JavaScript, PHP, Python, Ruby, Rust, Shell, TypeScript.

Adding more is a one-line edit in `src/lang.rs`.

## License

Dual-licensed under MIT or Apache-2.0, at your option.
