//! A resident `teq compiler watch` of the daemon: one per closure, run from the build's root, answering a
//! `build` with one JSON line (`src/watch.rs`). A first build's line carries the closure's whole
//! analysis and API, megabytes for a large closure, so the line is read whole but only its small
//! fields are parsed; the analysis and the API are merged into the resident's view of its
//! closure (`analysis.rs`) for the verbs that read them.

use super::analysis::{Fields, View};
use super::export::Key;
use crate::lsp::json::Json;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How long a resident asked to quit has before it is killed.
const QUIT_BOUND: Duration = Duration::from_secs(5);

pub struct Resident {
    pub key: Key,
    pub args: Vec<String>,
    /// The source roots that existed when it started: a declared root created since is not
    /// among its inputs, so a resident whose set changed is started anew.
    pub inputs: Vec<std::path::PathBuf>,
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    stderr: Arc<Mutex<String>>,
    /// The build a resident makes at its start is answered without being asked for.
    started: bool,
    /// The classes of each source file and their version 3 dependencies, as the answers so far
    /// leave them.
    pub analysis: View,
}

/// What a build answered.
pub struct Answer {
    pub ok: bool,
    /// The resident's first build, whose class files are written only where they differ from
    /// those on disk.
    pub first: bool,
    pub errors: Vec<Json>,
    pub warnings: Vec<Json>,
    /// The class files written, relative to the class directory.
    pub changed: Vec<String>,
    pub classes: usize,
    pub ms: f64,
    pub fallback: Option<String>,
    /// What the build says beside its diagnostics (the overlays' refused mappings).
    pub notes: Vec<String>,
}

impl Resident {
    /// Starts the resident from `root` as `teq @<file>`, its arguments written to the file, which
    /// stays for its life.
    pub fn start(exe: &Path, root: &Path, key: Key, args: Vec<String>, file: &Path, inputs: Vec<std::path::PathBuf>) -> Result<Resident, String> {
        crate::argfile::write(file, &args)?;
        let mut child = Command::new(exe)
            .arg(crate::argfile::at(file))
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("cannot start {}: {}", exe.display(), e))?;
        let stdin = child.stdin.take().expect("piped");
        let stdout = BufReader::new(child.stdout.take().expect("piped"));
        let mut err = child.stderr.take().expect("piped");
        let stderr = Arc::new(Mutex::new(String::new()));
        let tail = stderr.clone();
        crate::alloc::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = err.read(&mut buf) {
                if n == 0 {
                    break;
                }
                let mut t = tail.lock().unwrap_or_else(|e| e.into_inner());
                t.push_str(&String::from_utf8_lossy(&buf[..n]));
                if t.len() > 16384 {
                    let cut = t.len() - 8192;
                    let cut = (cut..t.len()).find(|&i| t.is_char_boundary(i)).unwrap_or(t.len());
                    t.drain(..cut);
                }
            }
        });
        Ok(Resident { key, args, inputs, child, stdin, stdout, stderr, started: false, analysis: View::default() })
    }

    /// A build of every file whose modification time moved; the first answers the resident's
    /// own first build.
    pub fn build(&mut self) -> Result<Answer, String> {
        let first = !self.started;
        if self.started {
            self.stdin.write_all(b"build\n").and_then(|()| self.stdin.flush()).map_err(|_| self.gone())?;
        }
        self.started = true;
        let mut line = String::new();
        if self.stdout.read_line(&mut line).map_err(|e| e.to_string())? == 0 {
            return Err(self.gone());
        }
        let fields = top_level_fields(line.trim_end());
        let parsed = |key: &str| fields.iter().find(|(k, _)| k == key).and_then(|(_, raw)| Json::parse(raw).ok());
        let raw = |key: &str| fields.iter().find(|(k, _)| k == key).map(|(_, raw)| *raw);
        let strings = |key: &str| parsed(key).map(|v| v.arr().iter().filter_map(Json::str).map(str::to_string).collect::<Vec<_>>()).unwrap_or_default();
        let removed = strings("removed");
        let merged = self.analysis.merge(&Fields {
            analysis: raw("analysis"),
            api: raw("api"),
            api_errors: raw("apiErrors"),
            incremental: parsed("incremental").and_then(|v| v.bool()) == Some(true),
            removed: &removed,
        });
        if let Err(e) = merged {
            self.analysis = View::unreadable(e);
        }
        Ok(Answer {
            ok: parsed("ok").and_then(|v| v.bool()) == Some(true),
            first,
            errors: parsed("errors").map(|v| v.arr().to_vec()).unwrap_or_default(),
            warnings: parsed("warnings").map(|v| v.arr().to_vec()).unwrap_or_default(),
            changed: strings("changed"),
            classes: parsed("modules").and_then(|v| v.num()).unwrap_or(0.0) as usize,
            ms: parsed("ms").and_then(|v| v.get("total").and_then(Json::num)).unwrap_or(0.0),
            fallback: parsed("fallback").and_then(|v| v.str().map(str::to_string)),
            notes: strings("notes"),
        })
    }

    /// Why the resident is gone: the end of what it wrote on stderr.
    fn gone(&mut self) -> String {
        let _ = self.child.try_wait();
        let tail = self.stderr.lock().unwrap_or_else(|e| e.into_inner()).trim().to_string();
        if tail.is_empty() {
            format!("the resident of {} ended", self.key)
        } else {
            format!("the resident of {} ended: {}", self.key, tail)
        }
    }

    /// Asks the resident to quit and kills it if it has not within `QUIT_BOUND`.
    pub fn stop(mut self) {
        let _ = self.stdin.write_all(b"quit\n").and_then(|()| self.stdin.flush());
        let until = Instant::now() + QUIT_BOUND;
        while Instant::now() < until {
            if !matches!(self.child.try_wait(), Ok(None)) {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The fields of a JSON object's text, each value's text as written, without parsing them.
pub fn top_level_fields(text: &str) -> Vec<(String, &str)> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut at = skip_ws(bytes, 0);
    if bytes.get(at) != Some(&b'{') {
        return out;
    }
    at += 1;
    loop {
        at = skip_ws(bytes, at);
        if bytes.get(at) != Some(&b'"') {
            return out;
        }
        let key_end = string_end(bytes, at);
        let Ok(Json::Str(key)) = Json::parse(&text[at..key_end]) else { return out };
        at = skip_ws(bytes, key_end);
        if bytes.get(at) != Some(&b':') {
            return out;
        }
        let start = skip_ws(bytes, at + 1);
        let end = value_end(bytes, start);
        out.push((key, &text[start..end]));
        at = skip_ws(bytes, end);
        match bytes.get(at) {
            Some(b',') => at += 1,
            _ => return out,
        }
    }
}

fn skip_ws(bytes: &[u8], mut at: usize) -> usize {
    while matches!(bytes.get(at), Some(b' ' | b'\t' | b'\n' | b'\r')) {
        at += 1;
    }
    at
}

/// The end of the string that starts at `at`, past its closing quote.
fn string_end(bytes: &[u8], mut at: usize) -> usize {
    at += 1;
    while at < bytes.len() {
        match bytes[at] {
            b'\\' => at += 2,
            b'"' => return at + 1,
            _ => at += 1,
        }
    }
    bytes.len()
}

/// The end of the value that starts at `at`.
fn value_end(bytes: &[u8], mut at: usize) -> usize {
    let mut depth = 0usize;
    while at < bytes.len() {
        match bytes[at] {
            b'"' => {
                at = string_end(bytes, at);
                if depth == 0 {
                    return at;
                }
                continue;
            }
            b'{' | b'[' => depth += 1,
            b'}' | b']' if depth == 0 => return at,
            b'}' | b']' => {
                depth -= 1;
                if depth == 0 {
                    return at + 1;
                }
            }
            b',' if depth == 0 => return at,
            _ => {}
        }
        at += 1;
    }
    at
}

/// The items of an array's text, each as written, their own nesting and strings passed over.
pub fn items(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut at = skip_ws(bytes, 0);
    if bytes.get(at) != Some(&b'[') {
        return out;
    }
    at = skip_ws(bytes, at + 1);
    while at < bytes.len() && bytes[at] != b']' {
        let end = value_end(bytes, at);
        out.push(text[at..end].trim_end());
        at = skip_ws(bytes, end);
        if bytes.get(at) == Some(&b',') {
            at = skip_ws(bytes, at + 1);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fields_of_an_answer_without_parsing_it() {
        let line = r#"{"ok":true,"changed":["a/B.class","c\"d]"],"modules":3,"ms":{"total":4.5,"x":[1,{"y":"}"}]},"api":{"k":[1,2]},"fallback":"first build"}"#;
        let fields = top_level_fields(line);
        let keys: Vec<&str> = fields.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, ["ok", "changed", "modules", "ms", "api", "fallback"]);
        assert_eq!(fields[2].1, "3");
        assert_eq!(fields[4].1, r#"{"k":[1,2]}"#);
        assert_eq!(items(fields[1].1), [r#""a/B.class""#, r#""c\"d]""#]);
        assert!(items("[]").is_empty());
        assert_eq!(items("[ {\"a\": [1, 2]} , 3 ]"), ["{\"a\": [1, 2]}", "3"]);
    }
}
