//! `TEQ_BENCH_SLOW=<percent>`: a slowdown of the compiler planted for the validation of the gate's
//! timing lines (docs/DEVELOPING.md, "The timing lines on a machine"), a test-only switch that
//! nothing else reads. The type phase spins for that percentage of its own time, and the output
//! reads `System.nanoTime()` through a clock that spins for that percentage of the time since its
//! previous reading, which makes the timed loop of bench/runtime's programs (one reading before an
//! iteration's work, one after) slower by that much on every target.

use std::sync::OnceLock;
use std::time::{Duration, Instant};

pub fn percent() -> Option<u64> {
    static PERCENT: OnceLock<Option<u64>> = OnceLock::new();
    *PERCENT.get_or_init(|| std::env::var("TEQ_BENCH_SLOW").ok().and_then(|v| v.parse().ok()).filter(|&p| p > 0))
}

pub fn spin_type_phase(started: Instant) {
    let Some(p) = percent() else { return };
    let until = Instant::now() + Duration::from_secs_f64(started.elapsed().as_secs_f64() * p as f64 / 100.0);
    while Instant::now() < until {
        std::hint::spin_loop();
    }
}

/// The source with its clock readings planted, unchanged without the switch.
pub fn plant_clock(text: String, file: usize) -> String {
    match percent() {
        Some(p) => planted(text, file, p),
        None => text,
    }
}

const READING: &str = "System.nanoTime()";
const QUALIFIERS: [&str; 3] = ["_root_.java.lang.", "java.lang.", ""];

/// The source with every reading of the clock in code, bare or qualified by `java.lang.`, read
/// through a clock of the file's own that spins for `p`% of the time since its previous reading;
/// a reading in a string, a character literal or a comment is text and stays.
fn planted(text: String, file: usize, p: u64) -> String {
    let spans = clock_readings(&text);
    if spans.is_empty() {
        return text;
    }
    let clock = format!("TeqBenchSlowClock{}", file);
    let mut out = String::with_capacity(text.len() + 300);
    let mut at = 0;
    for (start, end) in spans {
        out.push_str(&text[at..start]);
        out.push_str(&clock);
        out.push_str(".nanoTime()");
        at = end;
    }
    out.push_str(&text[at..]);
    out.push_str(&format!(
        "\n\nobject {clock}:\n  private var last = 0L\n  def nanoTime(): Long =\n    val now = java.lang.{READING}\n    if last != 0L then\n      val until = now + (now - last) * {p}L / 100L\n      while java.lang.{READING} < until do ()\n    last = java.lang.{READING}\n    last\n"
    ));
    out
}

/// The spans of the clock readings outside string and character literals and comments.
fn clock_readings(text: &str) -> Vec<(usize, usize)> {
    let b = text.as_bytes();
    let mut spans = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i..].starts_with(b"//") {
            i = text[i..].find('\n').map_or(b.len(), |n| i + n);
        } else if b[i..].starts_with(b"/*") {
            i = end_of_block_comment(b, i);
        } else if b[i..].starts_with(b"\"\"\"") {
            i = text[i + 3..].find("\"\"\"").map_or(b.len(), |n| i + 3 + n + 3);
            while i < b.len() && b[i] == b'"' {
                i += 1;
            }
        } else if b[i] == b'"' {
            i = end_of_string(b, i);
        } else if b[i] == b'\'' && b.get(i + 1) == Some(&b'\\') {
            i = text.get(i + 3..).and_then(|rest| rest.find('\'')).map_or(b.len(), |n| i + 3 + n + 1);
        } else if b[i] == b'\'' && b.get(i + 2) == Some(&b'\'') {
            i += 3;
        } else if is_ident(b[i]) && (i == 0 || !(is_ident(b[i - 1]) || b[i - 1] == b'.')) {
            match QUALIFIERS.iter().map(|q| format!("{}{}", q, READING)).find(|r| b[i..].starts_with(r.as_bytes())) {
                Some(r) => {
                    spans.push((i, i + r.len()));
                    i += r.len();
                }
                None => {
                    while i < b.len() && is_ident(b[i]) {
                        i += 1;
                    }
                }
            }
        } else {
            i += 1;
        }
    }
    spans
}

fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'$'
}

/// The end of a string from its opening quote: escapes skipped, and an interpolation's `${...}`
/// taken whole with its braces, so that a quote inside it does not close the string.
fn end_of_string(b: &[u8], start: usize) -> usize {
    let mut i = start + 1;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            b'"' => return i + 1,
            b'\n' => return i,
            b'$' if b.get(i + 1) == Some(&b'{') => {
                let mut depth = 0;
                while i < b.len() {
                    match b[i] {
                        b'{' => depth += 1,
                        b'}' => {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        b'"' => i = end_of_string(b, i) - 1,
                        _ => {}
                    }
                    i += 1;
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    b.len()
}

/// The end of a block comment, which nests in Scala.
fn end_of_block_comment(b: &[u8], start: usize) -> usize {
    let mut depth = 0;
    let mut i = start;
    while i < b.len() {
        if b[i..].starts_with(b"/*") {
            depth += 1;
            i += 2;
        } else if b[i..].starts_with(b"*/") {
            depth -= 1;
            i += 2;
            if depth == 0 {
                return i;
            }
        } else {
            i += 1;
        }
    }
    b.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn readings(text: &str) -> Vec<&str> {
        clock_readings(text).into_iter().map(|(s, e)| &text[s..e]).collect()
    }

    #[test]
    fn a_reading_in_code_is_planted_and_its_qualified_forms_whole() {
        let text = "val a = System.nanoTime()\nval b = java.lang.System.nanoTime()\nval c = _root_.java.lang.System.nanoTime()\n";
        assert_eq!(readings(text), vec!["System.nanoTime()", "java.lang.System.nanoTime()", "_root_.java.lang.System.nanoTime()"]);
        let out = planted(text.to_string(), 3, 25);
        assert!(out.starts_with("val a = TeqBenchSlowClock3.nanoTime()\nval b = TeqBenchSlowClock3.nanoTime()\nval c = TeqBenchSlowClock3.nanoTime()\n"));
        assert!(out.contains("object TeqBenchSlowClock3:") && out.contains("* 25L / 100L"));
    }

    #[test]
    fn a_reading_in_a_literal_or_a_comment_is_text() {
        let text = concat!(
            "println(\"System.nanoTime() is the clock\")\n",
            "println(s\"at ${\"System.nanoTime()\"} and \\\"System.nanoTime()\\\"\")\n",
            "val raw = \"\"\"System.nanoTime() \"quoted\" \"\"\"\n",
            "// System.nanoTime() in a line comment\n",
            "/* System.nanoTime() /* nested System.nanoTime() */ still a comment System.nanoTime() */\n",
            "val q = '\"'\nval e = '\\''\nval u = '\\u0041'\n",
            "val t = System.nanoTime()\n",
        );
        assert_eq!(readings(text), vec!["System.nanoTime()"]);
        let kept = &text[..text.find("val t = ").unwrap()];
        assert!(planted(text.to_string(), 0, 10).starts_with(&format!("{}val t = TeqBenchSlowClock0.nanoTime()", kept)));
    }

    #[test]
    fn another_qualifier_or_a_longer_name_is_not_the_clock() {
        let text = "val a = my.System.nanoTime()\nval b = MySystem.nanoTime()\nval c = System.nanoTimer()\n";
        assert_eq!(readings(text), Vec::<&str>::new());
        assert_eq!(planted(text.to_string(), 0, 10), text);
    }
}
