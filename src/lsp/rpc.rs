//! The base protocol of LSP: messages framed by a `Content-Length` header, read from a stream
//! whose reads may split a frame anywhere.

use std::io::{BufRead, Write};

/// The body of the next message; `None` at the end of the stream, `Err` for a frame whose
/// header cannot be read, after which the reader stands at the next header.
pub fn read_message(input: &mut impl BufRead) -> Option<Result<String, String>> {
    let mut length: Option<usize> = None;
    let mut malformed = None;
    let mut any = false;
    loop {
        let mut line = String::new();
        match input.read_line(&mut line) {
            Ok(0) => return any.then(|| Err("the stream ended inside a header".to_string())),
            Ok(_) => {}
            Err(e) => return Some(Err(format!("cannot read a header: {}", e))),
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            if !any {
                continue;
            }
            break;
        }
        any = true;
        match line.split_once(':') {
            Some((name, value)) if name.trim().eq_ignore_ascii_case("content-length") => match value.trim().parse::<usize>() {
                Ok(n) => length = Some(n),
                Err(_) => malformed = Some(format!("malformed Content-Length: {}", value.trim())),
            },
            Some(_) => {}
            None => malformed = Some(format!("malformed header line: {}", line)),
        }
    }
    let Some(n) = length else { return Some(Err(malformed.unwrap_or_else(|| "a header without Content-Length".to_string()))) };
    let mut body = vec![0u8; n];
    if input.read_exact(&mut body).is_err() {
        return None;
    }
    match malformed {
        Some(msg) => Some(Err(msg)),
        None => Some(String::from_utf8(body).map_err(|_| "a message that is not UTF-8".to_string())),
    }
}

pub fn write_message(out: &mut impl Write, body: &str) {
    let _ = write!(out, "Content-Length: {}\r\n\r\n{}", body.len(), body);
    let _ = out.flush();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames() {
        let text = "Content-Length: 2\r\n\r\n{}Content-Type: x\r\nContent-Length: 3\r\n\r\n[1]";
        let mut r = std::io::BufReader::new(text.as_bytes());
        assert_eq!(read_message(&mut r), Some(Ok("{}".to_string())));
        assert_eq!(read_message(&mut r), Some(Ok("[1]".to_string())));
        assert_eq!(read_message(&mut r), None);
    }

    #[test]
    fn bad_header_then_good() {
        let text = "Content-Length: x\r\n\r\nContent-Length: 2\r\n\r\n{}";
        let mut r = std::io::BufReader::new(text.as_bytes());
        assert!(matches!(read_message(&mut r), Some(Err(_))));
        assert_eq!(read_message(&mut r), Some(Ok("{}".to_string())));
    }
}
