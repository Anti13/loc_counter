use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

const SMALL_FILE_THRESHOLD: u64 = 64 * 1024;
const CHUNK_SIZE: usize = 64 * 1024;
// Git's binary-detection heuristic: scan only the first 8 KB for null bytes.
const BINARY_CHECK_BYTES: usize = 8 * 1024;

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
        let mut buffer = Vec::with_capacity(size as usize);
        file.read_to_end(&mut buffer)?;
        Ok(count_buffer(&buffer))
    } else {
        count_streaming(&mut file)
    }
}

fn count_buffer(buffer: &[u8]) -> LineCounts {
    if buffer.is_empty() {
        return LineCounts::default();
    }
    if looks_like_binary(buffer) {
        return LineCounts::binary();
    }
    let newlines = bytecount::count(buffer, b'\n') as u64;
    let lines = newlines + unterminated_line_adjustment(buffer.last().copied());
    LineCounts {
        lines,
        binary: false,
    }
}

fn count_streaming<R: Read>(reader: &mut R) -> io::Result<LineCounts> {
    let mut buffer = vec![0u8; CHUNK_SIZE];
    let mut total_lines: u64 = 0;
    let mut last_byte: Option<u8> = None;
    let mut is_first_chunk = true;

    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        let chunk = &buffer[..bytes_read];

        if is_first_chunk {
            is_first_chunk = false;
            if looks_like_binary(chunk) {
                return Ok(LineCounts::binary());
            }
        }

        total_lines += bytecount::count(chunk, b'\n') as u64;
        last_byte = Some(chunk[bytes_read - 1]);
    }

    total_lines += unterminated_line_adjustment(last_byte);

    Ok(LineCounts {
        lines: total_lines,
        binary: false,
    })
}

/// Treats the input as binary if a null byte appears in the first
/// `BINARY_CHECK_BYTES` of it. Same heuristic Git uses.
fn looks_like_binary(first_chunk: &[u8]) -> bool {
    let head = &first_chunk[..first_chunk.len().min(BINARY_CHECK_BYTES)];
    memchr::memchr(0, head).is_some()
}

/// If the file's last byte was not `\n`, the trailing line is unterminated and
/// hasn't been counted yet — return 1 to add it. Otherwise 0.
fn unterminated_line_adjustment(last_byte: Option<u8>) -> u64 {
    match last_byte {
        Some(byte) if byte != b'\n' => 1,
        _ => 0,
    }
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

    #[test]
    fn unterminated_line_adjustment_is_one_for_nonnewline_byte() {
        assert_eq!(unterminated_line_adjustment(Some(b'a')), 1);
    }

    #[test]
    fn unterminated_line_adjustment_is_zero_for_newline() {
        assert_eq!(unterminated_line_adjustment(Some(b'\n')), 0);
    }

    #[test]
    fn unterminated_line_adjustment_is_zero_for_empty() {
        assert_eq!(unterminated_line_adjustment(None), 0);
    }

    #[test]
    fn looks_like_binary_detects_null() {
        assert!(looks_like_binary(b"abc\0def"));
    }

    #[test]
    fn looks_like_binary_returns_false_for_text() {
        assert!(!looks_like_binary(b"hello world\n"));
    }

    #[test]
    fn single_newline_byte_is_one_line() {
        let f = write_tmp(b"\n");
        assert_eq!(count_file(f.path()).unwrap().lines, 1);
    }

    #[test]
    fn consecutive_blank_lines_are_counted() {
        let f = write_tmp(b"a\n\n\nb\n");
        assert_eq!(count_file(f.path()).unwrap().lines, 4);
    }

    #[test]
    fn crlf_line_endings_count_correctly() {
        let f = write_tmp(b"a\r\nb\r\nc\r\n");
        assert_eq!(count_file(f.path()).unwrap().lines, 3);
    }

    #[test]
    fn cr_only_line_endings_are_treated_as_one_unterminated_line() {
        // Old-Mac CR-only line endings are not handled specially; this is a
        // documented limitation. Lock in the current behavior.
        let f = write_tmp(b"a\rb\rc");
        assert_eq!(count_file(f.path()).unwrap().lines, 1);
    }

    #[test]
    fn file_just_below_threshold_uses_buffer_path() {
        let mut data = vec![b'x'; SMALL_FILE_THRESHOLD as usize - 1];
        data.push(b'\n');
        let f = write_tmp(&data);
        assert_eq!(count_file(f.path()).unwrap().lines, 1);
    }

    #[test]
    fn file_just_above_threshold_uses_streaming_path() {
        let mut data = vec![b'x'; SMALL_FILE_THRESHOLD as usize + 1];
        data.push(b'\n');
        let f = write_tmp(&data);
        assert_eq!(count_file(f.path()).unwrap().lines, 1);
    }

    #[test]
    fn streaming_path_detects_binary_in_first_chunk() {
        // > SMALL_FILE_THRESHOLD bytes forces the streaming path; null near
        // the start triggers the binary heuristic on the first read.
        let mut data = vec![b'a'; SMALL_FILE_THRESHOLD as usize + 1024];
        data[100] = 0;
        let f = write_tmp(&data);
        assert!(count_file(f.path()).unwrap().binary);
    }

    #[test]
    fn null_byte_past_binary_check_window_is_not_flagged() {
        // Match Git's heuristic: only the first BINARY_CHECK_BYTES are scanned
        // for nulls. A null past that window must not flag the file as binary.
        let total_size = SMALL_FILE_THRESHOLD as usize + 1024;
        let mut data = vec![b'a'; total_size];
        let null_position = BINARY_CHECK_BYTES + 1024;
        data[null_position] = 0;
        let f = write_tmp(&data);
        assert!(
            !count_file(f.path()).unwrap().binary,
            "null at byte {null_position} should not trigger binary detection"
        );
    }
}
