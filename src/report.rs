use std::collections::BTreeMap;
use std::io::{self, Write};

use crate::walk::{LangStats, ScanResult};

pub fn print(result: &ScanResult, mut out: impl Write) -> io::Result<()> {
    let sorted: BTreeMap<&str, LangStats> = result
        .per_language
        .iter()
        .map(|(lang, stats)| (lang.name(), *stats))
        .collect();

    let total_files: u64 = sorted.values().map(|s| s.files).sum();
    let total_lines: u64 = sorted.values().map(|s| s.lines).sum();

    let lang_w = sorted
        .keys()
        .map(|n| n.len())
        .max()
        .unwrap_or(0)
        .max("Language".len())
        .max("Total".len());
    let files_w = format_num(total_files).len().max("Files".len());
    let lines_w = format_num(total_lines).len().max("Lines".len());

    let sep_lang = "-".repeat(lang_w);
    let sep_files = "-".repeat(files_w);
    let sep_lines = "-".repeat(lines_w);

    writeln!(
        out,
        "{:<lw$}  {:>fw$}  {:>llw$}",
        "Language",
        "Files",
        "Lines",
        lw = lang_w,
        fw = files_w,
        llw = lines_w
    )?;
    writeln!(out, "{sep_lang}  {sep_files}  {sep_lines}")?;

    for (name, stats) in &sorted {
        writeln!(
            out,
            "{:<lw$}  {:>fw$}  {:>llw$}",
            name,
            format_num(stats.files),
            format_num(stats.lines),
            lw = lang_w,
            fw = files_w,
            llw = lines_w
        )?;
    }

    writeln!(out, "{sep_lang}  {sep_files}  {sep_lines}")?;
    writeln!(
        out,
        "{:<lw$}  {:>fw$}  {:>llw$}",
        "Total",
        format_num(total_files),
        format_num(total_lines),
        lw = lang_w,
        fw = files_w,
        llw = lines_w
    )?;

    if result.skipped_binary > 0 || result.read_errors > 0 {
        writeln!(
            out,
            "\nSkipped {} binary file(s); {} read error(s).",
            result.skipped_binary, result.read_errors
        )?;
    }

    Ok(())
}

fn format_num(n: u64) -> String {
    let s = n.to_string();
    let bytes = s.as_bytes();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    let first = bytes.len() % 3;
    if first > 0 {
        out.push_str(&s[..first]);
    }
    for (i, chunk) in bytes[first..].chunks(3).enumerate() {
        if i > 0 || first > 0 {
            out.push(',');
        }
        out.push_str(std::str::from_utf8(chunk).unwrap());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::Language;

    #[test]
    fn formats_thousands() {
        assert_eq!(format_num(0), "0");
        assert_eq!(format_num(7), "7");
        assert_eq!(format_num(123), "123");
        assert_eq!(format_num(1_234), "1,234");
        assert_eq!(format_num(12_345), "12,345");
        assert_eq!(format_num(123_456), "123,456");
        assert_eq!(format_num(1_234_567), "1,234,567");
        assert_eq!(format_num(1_000_000), "1,000,000");
    }

    #[test]
    fn report_renders_with_total() {
        let mut result = ScanResult::default();
        result.per_language.insert(
            Language::Rust,
            LangStats {
                files: 2,
                lines: 100,
            },
        );
        result.per_language.insert(
            Language::C,
            LangStats {
                files: 1,
                lines: 50,
            },
        );
        let mut buf = Vec::new();
        print(&result, &mut buf).unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("Rust"));
        assert!(s.contains("C"));
        assert!(s.contains("Total"));
        assert!(s.contains("150"));
    }
}
