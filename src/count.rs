use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

const SMALL_FILE_THRESHOLD: u64 = 64 * 1024;
const CHUNK_SIZE: usize = 64 * 1024;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct LineCounts {
    pub lines: u64,
    pub binary: bool,
}

impl LineCounts {
    pub fn binary() -> Self {
        Self {
            lines: 0,
            binary: true,
        }
    }
}

pub fn count_file(path: &Path) -> io::Result<LineCounts> {
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();

    if size <= SMALL_FILE_THRESHOLD {
        let mut buf = Vec::with_capacity(size as usize);
        file.read_to_end(&mut buf)?;
        Ok(count_buffer(&buf))
    } else {
        count_streaming(&mut file)
    }
}

fn count_buffer(buf: &[u8]) -> LineCounts {
    if buf.is_empty() {
        return LineCounts::default();
    }
    if memchr::memchr(0, buf).is_some() {
        return LineCounts::binary();
    }
    let mut lines = bytecount::count(buf, b'\n') as u64;
    if *buf.last().unwrap() != b'\n' {
        lines += 1;
    }
    LineCounts {
        lines,
        binary: false,
    }
}

fn count_streaming<R: Read>(reader: &mut R) -> io::Result<LineCounts> {
    let mut buf = vec![0u8; CHUNK_SIZE];
    let mut total: u64 = 0;
    let mut last_byte: Option<u8> = None;
    let mut first = true;

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        let chunk = &buf[..n];

        if first {
            first = false;
            // Limit the binary-check scan to the first ~8 KB to match Git's heuristic.
            let head = &chunk[..chunk.len().min(8 * 1024)];
            if memchr::memchr(0, head).is_some() {
                return Ok(LineCounts::binary());
            }
        }

        total += bytecount::count(chunk, b'\n') as u64;
        last_byte = Some(chunk[n - 1]);
    }

    if let Some(b) = last_byte
        && b != b'\n'
    {
        total += 1;
    }

    Ok(LineCounts {
        lines: total,
        binary: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_tmp(bytes: &[u8]) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(bytes).unwrap();
        f.flush().unwrap();
        f
    }

    #[test]
    fn empty_file_is_zero_lines() {
        let f = write_tmp(b"");
        let c = count_file(f.path()).unwrap();
        assert_eq!(
            c,
            LineCounts {
                lines: 0,
                binary: false
            }
        );
    }

    #[test]
    fn trailing_newline_does_not_double_count() {
        let f = write_tmp(b"a\nb\nc\n");
        assert_eq!(count_file(f.path()).unwrap().lines, 3);
    }

    #[test]
    fn missing_trailing_newline_counts_final_line() {
        let f = write_tmp(b"a\nb\nc");
        assert_eq!(count_file(f.path()).unwrap().lines, 3);
    }

    #[test]
    fn single_line_no_newline() {
        let f = write_tmp(b"hello");
        assert_eq!(count_file(f.path()).unwrap().lines, 1);
    }

    #[test]
    fn binary_file_is_flagged() {
        let f = write_tmp(b"text\0more");
        let c = count_file(f.path()).unwrap();
        assert!(c.binary);
    }

    #[test]
    fn large_file_streams_correctly() {
        // Write > SMALL_FILE_THRESHOLD bytes to force the streaming path.
        let mut data = Vec::with_capacity((SMALL_FILE_THRESHOLD as usize) + 1024);
        for _ in 0..((SMALL_FILE_THRESHOLD as usize / 4) + 100) {
            data.extend_from_slice(b"abc\n");
        }
        let expected_lines = (data.iter().filter(|&&b| b == b'\n').count()) as u64;
        let f = write_tmp(&data);
        assert_eq!(count_file(f.path()).unwrap().lines, expected_lines);
    }
}
