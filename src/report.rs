use std::collections::BTreeMap;
use std::io::{self, Write};

use crate::walk::{LanguageStats, ScanResult};

pub fn print(result: &ScanResult, mut out: impl Write) -> io::Result<()> {
    let rows: BTreeMap<&str, LanguageStats> = result
        .per_language
        .iter()
        .map(|(language, stats)| (language.name(), *stats))
        .collect();

    let total_files: u64 = rows.values().map(|stats| stats.files).sum();
    let total_lines: u64 = rows.values().map(|stats| stats.lines).sum();

    let widths = ColumnWidths::compute(&rows, total_files, total_lines);
    let separator = widths.separator_line();

    write_header(&mut out, &widths)?;
    writeln!(out, "{separator}")?;
    for (name, stats) in &rows {
        write_row(&mut out, &widths, name, stats.files, stats.lines)?;
    }
    writeln!(out, "{separator}")?;
    write_row(&mut out, &widths, "Total", total_files, total_lines)?;

    if result.skipped_binary > 0 || result.read_errors > 0 {
        writeln!(
            out,
            "\nSkipped {} binary file(s); {} read error(s).",
            result.skipped_binary, result.read_errors
        )?;
    }

    Ok(())
}

struct ColumnWidths {
    language: usize,
    files: usize,
    lines: usize,
}

impl ColumnWidths {
    fn compute(rows: &BTreeMap<&str, LanguageStats>, total_files: u64, total_lines: u64) -> Self {
        let language = rows
            .keys()
            .map(|name| name.len())
            .max()
            .unwrap_or(0)
            .max("Language".len())
            .max("Total".len());
        let files = format_num(total_files).len().max("Files".len());
        let lines = format_num(total_lines).len().max("Lines".len());
        Self {
            language,
            files,
            lines,
        }
    }

    fn separator_line(&self) -> String {
        format!(
            "{}  {}  {}",
            "-".repeat(self.language),
            "-".repeat(self.files),
            "-".repeat(self.lines),
        )
    }
}

fn write_header(out: &mut impl Write, widths: &ColumnWidths) -> io::Result<()> {
    writeln!(
        out,
        "{:<language_width$}  {:>files_width$}  {:>lines_width$}",
        "Language",
        "Files",
        "Lines",
        language_width = widths.language,
        files_width = widths.files,
        lines_width = widths.lines,
    )
}

fn write_row(
    out: &mut impl Write,
    widths: &ColumnWidths,
    name: &str,
    files: u64,
    lines: u64,
) -> io::Result<()> {
    writeln!(
        out,
        "{:<language_width$}  {:>files_width$}  {:>lines_width$}",
        name,
        format_num(files),
        format_num(lines),
        language_width = widths.language,
        files_width = widths.files,
        lines_width = widths.lines,
    )
}

fn format_num(value: u64) -> String {
    let digits = value.to_string();
    let bytes = digits.as_bytes();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    let leading_group_len = bytes.len() % 3;
    if leading_group_len > 0 {
        out.push_str(&digits[..leading_group_len]);
    }
    for (group_index, group) in bytes[leading_group_len..].chunks(3).enumerate() {
        if group_index > 0 || leading_group_len > 0 {
            out.push(',');
        }
        out.push_str(std::str::from_utf8(group).unwrap());
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
            LanguageStats {
                files: 2,
                lines: 100,
            },
        );
        result.per_language.insert(
            Language::C,
            LanguageStats {
                files: 1,
                lines: 50,
            },
        );
        let mut buf = Vec::new();
        print(&result, &mut buf).unwrap();
        let rendered = String::from_utf8(buf).unwrap();
        assert!(rendered.contains("Rust"));
        assert!(rendered.contains("C"));
        assert!(rendered.contains("Total"));
        assert!(rendered.contains("150"));
    }
}
