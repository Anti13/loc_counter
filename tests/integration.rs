use std::fs;

use loc_counter::lang::Language;
use loc_counter::walk::{WalkOptions, scan};

#[test]
fn aggregates_per_language_across_a_tree() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    fs::write(root.join("a.rs"), b"fn main() {}\nfn b() {}\n").unwrap();
    fs::write(root.join("b.rs"), b"// rust\n").unwrap();
    fs::write(root.join("c.c"), b"#include <stdio.h>\nint main(){}\n").unwrap();
    fs::write(root.join("d.cpp"), b"int x = 1;\nint y = 2;\nint z = 3;\n").unwrap();
    // Unknown extension — must be ignored.
    fs::write(root.join("notes.txt"), b"hello\nworld\n").unwrap();
    // Binary file — must be skipped.
    fs::write(root.join("blob.rs"), b"a\0b").unwrap();

    let result = scan(
        root,
        WalkOptions {
            respect_ignore: false,
            include_hidden: true,
        },
    );

    let rust = result
        .per_language
        .get(&Language::Rust)
        .copied()
        .unwrap_or_default();
    assert_eq!(
        rust.files, 2,
        "should count two valid Rust files (binary one excluded)"
    );
    assert_eq!(rust.lines, 3, "a.rs has 2 lines + b.rs has 1 line");

    let c = result
        .per_language
        .get(&Language::C)
        .copied()
        .unwrap_or_default();
    assert_eq!(c.files, 1);
    assert_eq!(c.lines, 2);

    let cpp = result
        .per_language
        .get(&Language::Cpp)
        .copied()
        .unwrap_or_default();
    assert_eq!(cpp.files, 1);
    assert_eq!(cpp.lines, 3);

    assert_eq!(result.skipped_binary, 1);

    // Unknown extensions can't sneak in: the enum has no "Other" variant.
    assert_eq!(result.per_language.len(), 3);
}

#[test]
fn empty_tree_produces_zero_totals() {
    let tmp = tempfile::tempdir().unwrap();
    let result = scan(
        tmp.path(),
        WalkOptions {
            respect_ignore: false,
            include_hidden: true,
        },
    );
    assert!(result.per_language.is_empty());
    assert_eq!(result.skipped_binary, 0);
    assert_eq!(result.read_errors, 0);
}
