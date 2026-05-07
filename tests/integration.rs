use std::fs;
use std::path::Path;

use loc_counter::lang::Language;
use loc_counter::walk::{LanguageStats, ScanResult, WalkOptions, scan};

/// Default options with both ignore-files and hidden-file filters disabled,
/// so a test sees every file it created unless it explicitly opts in.
fn permissive_options() -> WalkOptions {
    WalkOptions {
        respect_ignore: false,
        include_hidden: true,
    }
}

fn stats_for(result: &ScanResult, language: Language) -> LanguageStats {
    result
        .per_language
        .get(&language)
        .copied()
        .unwrap_or_default()
}

fn write(path: impl AsRef<Path>, contents: &[u8]) {
    fs::write(path, contents).unwrap();
}

#[test]
fn aggregates_per_language_across_a_tree() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    write(root.join("a.rs"), b"fn main() {}\nfn b() {}\n");
    write(root.join("b.rs"), b"// rust\n");
    write(root.join("c.c"), b"#include <stdio.h>\nint main(){}\n");
    write(root.join("d.cpp"), b"int x = 1;\nint y = 2;\nint z = 3;\n");
    write(root.join("notes.txt"), b"hello\nworld\n"); // unknown ext — skipped
    write(root.join("blob.rs"), b"a\0b"); // binary — skipped

    let result = scan(root, permissive_options());

    let rust = stats_for(&result, Language::Rust);
    assert_eq!(rust.files, 2, "two valid Rust files (binary one excluded)");
    assert_eq!(rust.lines, 3, "a.rs has 2 lines + b.rs has 1 line");

    let c = stats_for(&result, Language::C);
    assert_eq!(c.files, 1);
    assert_eq!(c.lines, 2);

    let cpp = stats_for(&result, Language::Cpp);
    assert_eq!(cpp.files, 1);
    assert_eq!(cpp.lines, 3);

    assert_eq!(result.skipped_binary, 1);
    assert_eq!(result.per_language.len(), 3);
}

#[test]
fn empty_tree_produces_zero_totals() {
    let tmp = tempfile::tempdir().unwrap();
    let result = scan(tmp.path(), permissive_options());
    assert!(result.per_language.is_empty());
    assert_eq!(result.skipped_binary, 0);
    assert_eq!(result.read_errors, 0);
}

#[test]
fn nested_directories_are_walked_recursively() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let deep = root.join("a").join("b").join("c");
    fs::create_dir_all(&deep).unwrap();

    write(root.join("top.rs"), b"// top\n");
    write(root.join("a").join("mid.rs"), b"// mid 1\n// mid 2\n");
    write(deep.join("deep.rs"), b"// deep\n");

    let result = scan(root, permissive_options());
    let rust = stats_for(&result, Language::Rust);
    assert_eq!(rust.files, 3);
    assert_eq!(rust.lines, 4);
}

#[test]
fn ignore_file_excludes_matching_files_when_respected() {
    // We use `.ignore` (not `.gitignore`) because the `ignore` crate respects
    // it unconditionally — `.gitignore` files only kick in inside a real git
    // repository, which would be fragile to set up in a test.
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    write(root.join(".ignore"), b"excluded.rs\n");
    write(root.join("kept.rs"), b"// kept\n");
    write(root.join("excluded.rs"), b"// excluded\n");

    let result = scan(
        root,
        WalkOptions {
            respect_ignore: true,
            include_hidden: false,
        },
    );

    let rust = stats_for(&result, Language::Rust);
    assert_eq!(rust.files, 1, "only kept.rs should be counted");
    assert_eq!(rust.lines, 1);
}

#[test]
fn no_ignore_flag_includes_files_an_ignore_file_would_exclude() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    write(root.join(".ignore"), b"excluded.rs\n");
    write(root.join("kept.rs"), b"// kept\n");
    write(root.join("excluded.rs"), b"// excluded\n");

    let result = scan(
        root,
        WalkOptions {
            respect_ignore: false,
            include_hidden: false,
        },
    );

    let rust = stats_for(&result, Language::Rust);
    assert_eq!(rust.files, 2, "no_ignore should include both files");
}

#[test]
fn hidden_files_are_excluded_by_default() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    write(root.join("visible.rs"), b"// v\n");
    write(root.join(".hidden.rs"), b"// h\n");

    let result = scan(
        root,
        WalkOptions {
            respect_ignore: false,
            include_hidden: false,
        },
    );

    let rust = stats_for(&result, Language::Rust);
    assert_eq!(rust.files, 1, "hidden file should be excluded");
}

#[test]
fn hidden_flag_includes_dotfiles() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();

    write(root.join("visible.rs"), b"// v\n");
    write(root.join(".hidden.rs"), b"// h\n");

    let result = scan(
        root,
        WalkOptions {
            respect_ignore: false,
            include_hidden: true,
        },
    );

    let rust = stats_for(&result, Language::Rust);
    assert_eq!(rust.files, 2);
}

#[test]
fn hidden_directories_are_excluded_by_default() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let hidden_dir = root.join(".hidden");
    fs::create_dir(&hidden_dir).unwrap();

    write(root.join("visible.rs"), b"// v\n");
    write(hidden_dir.join("inside.rs"), b"// inside\n");

    let result = scan(
        root,
        WalkOptions {
            respect_ignore: false,
            include_hidden: false,
        },
    );

    let rust = stats_for(&result, Language::Rust);
    assert_eq!(
        rust.files, 1,
        "files under hidden dirs must also be excluded"
    );
}
