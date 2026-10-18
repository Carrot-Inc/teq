//! What a source file says before its code for the tools around the compiler: a script header
//! (`#!/usr/bin/env -S teq interp`), which scalac skips, and Scala CLI's `//> using` directives, of
//! which `teq interp` honours `file` and `files` (the sources a script includes) and the `-source`
//! among the scalac options of `option`, `options`, `scalacOption` and `scalacOptions`, and leaves
//! the others as the comments they are to scalac.

/// The length of the script header that opens `text`, zero for none: scalac's rule
/// (`ScriptSourceFile`): a text that starts with `#!` or `::#!` has a header that runs through the
/// first line starting with `!#` or `::!#`, its line end included, or else is its first line. The
/// lexer skips it as a comment, so that positions after it stay the file's.
pub fn script_header(text: &str) -> usize {
    if !(text.starts_with("#!") || text.starts_with("::#!")) {
        return 0;
    }
    let mut at = 0;
    while at < text.len() {
        let rest = &text[at..];
        let end = rest.find(['\n', '\r']).map_or(rest.len(), |i| i);
        let line = &rest[..end];
        let opener = line.strip_prefix("::").unwrap_or(line);
        // A closing line counts with its line end alone, as the JDK's pattern `(\r|\n|\r\n)` takes
        // the first alternative that matches.
        if at > 0 && opener.starts_with("!#") && end < rest.len() {
            return at + end + 1;
        }
        at += end + 1;
        if rest[end..].starts_with("\r\n") && at < text.len() {
            at += 1;
        }
    }
    text.find('\n').unwrap_or(text.len())
}

/// A `//> using file` or `//> using files` directive of a file's header: the line it stands on
/// (1-based) and its paths as written.
pub struct UsingFiles {
    pub line: usize,
    pub paths: Vec<String>,
}

/// A `//> using dep` directive, or another key of Scala CLI's `Dependency` handler that puts its jars on the
/// program's classpath (`dependency_key`), of a file's header: the line it stands on and its coordinates as
/// written (`org::name:version`), a quoted one decoded.
pub struct UsingDeps {
    pub line: usize,
    pub key: String,
    pub coordinates: Vec<String>,
}

/// What Scala CLI reports of a file's directives before it uses them: an error of its lexer (`Lexer`'s
/// `LexError`), which fails the build, or a deprecated key's warning (`DeprecatedDirectives`). Its line and
/// column (both 1-based, the column in characters), the width it underlines and the line's text.
pub struct Problem {
    pub error: bool,
    pub line: usize,
    pub col: usize,
    pub width: usize,
    pub line_text: String,
    pub msg: String,
}

impl Problem {
    /// The problem in the compiler's form: `<path>:<line>:<col>: error: <msg>`, the line and its marks.
    pub fn render(&self, path: &str) -> String {
        format!(
            "{}:{}:{}: {}: {}\n  {}\n  {}{}\n",
            path,
            self.line,
            self.col,
            if self.error { "error" } else { "warning" },
            self.msg,
            self.line_text,
            " ".repeat(self.col.saturating_sub(1)),
            "^".repeat(self.width.max(1))
        )
    }
}

/// The `file` and `files` directives among the directives that open `text`, as Scala CLI reads
/// them (`header`).
pub fn using_files(text: &str) -> Vec<UsingFiles> {
    header(text).0.into_iter().filter(|d| (d.key == "file" || d.key == "files") && !d.values.is_empty()).map(|d| UsingFiles { line: d.line, paths: d.values }).collect()
}

/// The source version the scalac options of the directives that open `text` name (`option`,
/// `options`, `scalacOption`, `scalacOptions`): `-source:<v>` or `-source <v>`, the last of them,
/// with the line it stands on.
pub fn using_source(text: &str) -> Option<(usize, String)> {
    let mut found = None;
    for d in header(text).0 {
        if !matches!(d.key.as_str(), "option" | "options" | "scalacOption" | "scalacOptions") {
            continue;
        }
        let mut values = d.values.iter();
        while let Some(v) = values.next() {
            if let Some(version) = v.strip_prefix("-source:") {
                found = Some((d.line, version.to_string()));
            } else if v == "-source" {
                if let Some(version) = values.next() {
                    found = Some((d.line, version.clone()));
                }
            }
        }
    }
    found
}

/// The dependency directives among the directives that open `text` (`header`), under every key that puts
/// their jars on the program's classpath (`dependency_key`).
pub fn using_deps(text: &str) -> Vec<UsingDeps> {
    header(text).0.into_iter().filter(|d| dependency_key(&d.key) && !d.values.is_empty()).map(|d| UsingDeps { line: d.line, key: d.key, coordinates: d.values }).collect()
}

/// The lexer's errors and the deprecated keys' warnings of the directives that open `text`, in their order.
pub fn problems(text: &str) -> Vec<Problem> {
    let (directives, mut found, code) = header(text);
    if let Some(code) = code {
        found.extend(ignored_after_code(text, code));
    }
    for d in &directives {
        if let Some(instead) = deprecated_key(&d.key) {
            found.push(Problem {
                error: false,
                line: d.line,
                col: d.col,
                width: d.width,
                line_text: d.line_text.clone(),
                msg: format!("Using '{}' is deprecated, use '{}' instead", d.key, instead),
            });
        }
    }
    found.sort_by_key(|p| (p.line, p.col));
    found
}

/// Whether `key` is one of Scala CLI's `Dependency` handler whose jars a run's classpath holds: the runtime
/// dependencies' (`dep`, `deps`, `dependencies`, the field's `dependency`, the deprecated `lib` and `libs`)
/// or the compile-only ones' (`compileOnly.dep` and its like, the field's `compileOnlyDependency`), a key
/// compared as `DirectiveHandler.normalizeName` makes it, its dashed parts camel-cased (`compile-only.dep`).
/// The test scope's (`test.dep`) and scalafix's (`scalafix.dep`) have no part in a run.
fn dependency_key(key: &str) -> bool {
    let mut parts = key.split('-');
    let mut name = parts.next().unwrap_or("").to_string();
    for part in parts {
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            name.extend(first.to_uppercase());
            name.push_str(chars.as_str());
        }
    }
    matches!(
        name.as_str(),
        "dependency" | "lib" | "libs" | "dep" | "deps" | "dependencies" | "compileOnlyDependency" | "compileOnly.lib" | "compileOnly.libs" | "compileOnly.dep" | "compileOnly.deps" | "compileOnly.dependencies"
    )
}

/// The key a deprecated dependency key stands for, as Scala CLI's `DeprecatedDirectives` names it in its
/// warning (the key as written, not normalised).
fn deprecated_key(key: &str) -> Option<&'static str> {
    match key {
        "lib" => Some("dep"),
        "libs" => Some("deps"),
        "compileOnly.lib" => Some("compileOnly.dep"),
        "compileOnly.libs" => Some("compileOnly.deps"),
        _ => None,
    }
}

/// A directive of a file's header: its line, key and values, the column its key starts at, the width from
/// there to the end of its last word, and the line's text.
struct Directive {
    line: usize,
    key: String,
    values: Vec<String>,
    col: usize,
    width: usize,
    line_text: String,
}

/// The directives that open `text`, as Scala CLI 1.17.1's `CommentExtractor.extract` finds them, line by line
/// after the script header (and a byte order mark): a blank line, a line comment (`//`, not `//>`) and a block
/// comment that starts a line are skipped, the comment whole, nested ones in it; a block comment's closing line
/// that holds anything but blanks after it, another comment included, is code from the comment's first line on. A
/// line starting `//> using` (blanks before it) is a directive, read by `parse_line`; a line that is `//>`, blanks
/// and `using` otherwise spelt is none, warned of, and ends the header after it; any other line is code. Each
/// directive its line, key and values; the lexer's and parser's errors and warnings; and where code starts.
fn header(text: &str) -> (Vec<Directive>, Vec<Problem>, Option<usize>) {
    let mut out = Vec::new();
    let mut problems = Vec::new();
    let bom = if text.starts_with('\u{feff}') { '\u{feff}'.len_utf8() } else { 0 };
    let mut pos = bom + script_header(&text[bom..]);
    // A header that is its first line ends before the line's end.
    if text[pos..].starts_with('\n') {
        pos += 1;
    }
    let mut code = None;
    while pos < text.len() {
        let (line_start, line_end, next) = line_at(text, pos);
        let content = &text[line_start..line_end];
        let trimmed = content.trim_start();
        if trimmed.is_empty() || (trimmed.starts_with("//") && !trimmed.starts_with("//>")) {
            pos = next;
            continue;
        }
        if trimmed.starts_with("/*") {
            let open = line_start + content.find("/*").unwrap_or(0);
            // An unterminated comment holds the rest of the file, and no code follows.
            let Some(end) = crate::lexer::block_comment_end(text.as_bytes(), open) else { break };
            let (_, close_end, after) = line_at(text, end);
            if text[end..close_end].chars().any(|c| !matches!(c, ' ' | '\t' | '\r')) {
                code = Some(line_start);
                break;
            }
            pos = after;
            continue;
        }
        let lead = content.len() - content.trim_start_matches([' ', '\t']).len();
        let line_text = content.strip_suffix('\r').unwrap_or(content);
        let directive = &line_text[lead..];
        let line = text[..line_start].matches('\n').count() + 1;
        // The column of a character of the line, past a byte order mark on the first.
        let col = |at: usize| text[line_start.max(bom)..at].chars().count() + 1;
        if directive.starts_with("//> using") {
            let base = col(line_start + lead + 4);
            let (found, errors) = parse_line(&directive[4..]);
            for (at, error, msg) in errors {
                problems.push(Problem { error, line, col: base + at, width: 1, line_text: line_text.to_string(), msg });
            }
            if let Some((key, values)) = found {
                let end = values.last().unwrap_or(&key).end;
                out.push(Directive {
                    line,
                    key: key.text,
                    values: values.into_iter().map(|w| w.text).collect(),
                    col: base + key.start,
                    width: end - key.start,
                    line_text: line_text.to_string(),
                });
            }
            pos = next;
            continue;
        }
        if misspelt(directive) {
            let msg = format!("Using directive must use the exact prefix `//> using`. Invalid prefix in: {}", directive.trim());
            problems.push(Problem { error: false, line, col: col(line_start + lead), width: 1, line_text: line_text.to_string(), msg });
            code = Some(next);
            break;
        }
        code = Some(line_start);
        break;
    }
    (out, problems, code)
}

/// The line of `text` that `at` stands on: where it starts, where it ends (before its line end) and where the next
/// one starts.
fn line_at(text: &str, at: usize) -> (usize, usize, usize) {
    let start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let end = text[at..].find('\n').map_or(text.len(), |i| at + i);
    (start, end, (end + 1).min(text.len()))
}

/// A `//>` line that `CommentExtractor`'s `UsingDirectiveRegex` takes for a directive, `//>\s+using(?:\s|$)`.
fn misspelt(line: &str) -> bool {
    let java_blank = |c: char| matches!(c, ' ' | '\t' | '\n' | '\x0b' | '\x0c' | '\r');
    let Some(rest) = line.strip_prefix("//>") else { return false };
    let after = rest.trim_start_matches(java_blank);
    after.len() < rest.len() && after.strip_prefix("using").is_some_and(|r| r.chars().next().is_none_or(java_blank))
}

/// The directives after the code starting at `code`, which Scala CLI's extractor ignores with a warning each: a
/// line starting `//> using`, or taken for a directive, block comments skipped.
fn ignored_after_code(text: &str, code: usize) -> Vec<Problem> {
    let mut found = Vec::new();
    let mut pos = code;
    while pos < text.len() {
        let (line_start, line_end, next) = line_at(text, pos);
        let content = &text[line_start..line_end];
        let trimmed = content.trim_start();
        if trimmed.starts_with("/*") {
            let open = line_start + content.find("/*").unwrap_or(0);
            let Some(end) = crate::lexer::block_comment_end(text.as_bytes(), open) else { break };
            pos = line_at(text, end).2;
            continue;
        }
        if trimmed.starts_with("//> using") || misspelt(trimmed) {
            let line_text = content.strip_suffix('\r').unwrap_or(content);
            found.push(Problem {
                error: false,
                line: text[..line_start].matches('\n').count() + 1,
                col: 1,
                width: 1,
                line_text: line_text.to_string(),
                msg: format!("Ignoring using directive found after Scala code: {}", line_text.trim()),
            });
        }
        pos = next;
    }
    found
}

/// A token of a directive line (Scala CLI's `Token`), its kind kept for the parser.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Using,
    Ident,
    StringLit,
    BoolLit,
    Comma,
    LexError,
}

/// A token of a directive line: its kind, its text (decoded, a lexer error's message) and the characters it spans
/// (from its first to past its last).
struct Word {
    kind: Kind,
    text: String,
    start: usize,
    end: usize,
}

/// A directive line past `//> ` as Scala CLI's `Parser.parse` reads its tokens: `using`, then a key that is an
/// identifier, then its values. A key of another kind (a string, a lexer error such as an empty or unterminated
/// backtick identifier) or none is an error, as is any other first token; among the values a lexer error is one,
/// and a comma between values and a value ending with one are warned of. The key and values where they stand; the
/// errors and warnings at the character each reports.
fn parse_line(text: &str) -> (Option<(Word, Vec<Word>)>, Vec<(usize, bool, String)>) {
    let tokens = values(text);
    let mut problems = Vec::new();
    let name = |k: Kind| format!("{:?}", k);
    match tokens.first() {
        Some(t) if t.kind == Kind::Using => {}
        Some(t) => {
            let msg = if t.kind == Kind::LexError { format!("Lexer error: {}", t.text) } else { format!("Unexpected token: {}", name(t.kind)) };
            problems.push((t.start, true, msg));
            return (None, problems);
        }
        None => return (None, problems),
    }
    let mut tokens = tokens.into_iter();
    let using = tokens.next().unwrap();
    let key = match tokens.next() {
        Some(k) if k.kind == Kind::Ident => k,
        Some(k) => {
            problems.push((k.start, true, format!("Expected a key after `using`, found: {}", name(k.kind))));
            return (None, problems);
        }
        None => {
            problems.push((using.start, true, "Expected a key after `using`".to_string()));
            return (None, problems);
        }
    };
    let mut values = Vec::new();
    for t in tokens {
        match t.kind {
            Kind::Comma if !values.is_empty() => problems.push((t.start, false, "Use of commas as separators is deprecated. Only whitespace is necessary.".to_string())),
            Kind::LexError => problems.push((t.start, true, format!("Lexer error: {}", t.text))),
            Kind::Ident if t.text.ends_with(',') => {
                problems.push((t.start, false, format!("Value '{}' ends with a comma — this is likely a typo from a double-comma sequence.", t.text)));
                values.push(t);
            }
            _ => values.push(t),
        }
    }
    (Some((key, values)), problems)
}

/// The tokens of a directive line past `//> `, as Scala CLI's `Lexer.tokenize` makes them, in its order.
fn values(text: &str) -> Vec<Word> {
    let chars: Vec<char> = text.chars().collect();
    // Scala CLI's `Lexer.isWhitespace`.
    let blank = |c: char| matches!(c, ' ' | '\t' | '\r' | '\n' | '\x0c');
    // A comma that separates: one before a blank or the end.
    let separator = |i: usize| chars[i] == ',' && chars.get(i + 1).is_none_or(|&c| blank(c));
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if blank(chars[i]) {
            i += 1;
            continue;
        }
        let start = i;
        if separator(i) {
            i += 1;
            out.push(Word { kind: Kind::Comma, text: ",".to_string(), start, end: i });
        } else if chars[i] == '"' {
            // A string, decoded as Scala CLI's lexer decodes it: `\n`, `\t`, `\r`, `\\` and `\"`, `\u` and the
            // four hexadecimal digits after it as `Integer.parseInt` reads them (a UTF-16 unit, a sign
            // allowed), any other escaped character itself, and a backslash that ends the line as written.
            // An invalid unicode escape is an error at its quote, the string read on; an unterminated string is
            // one in its place.
            let mut units: Vec<u16> = Vec::new();
            let push = |units: &mut Vec<u16>, c: char| units.extend_from_slice(c.encode_utf16(&mut [0; 2]));
            let mut closed = false;
            i += 1;
            while i < chars.len() {
                match chars[i] {
                    '"' => {
                        closed = true;
                        i += 1;
                        break;
                    }
                    '\\' if i + 1 < chars.len() => {
                        i += 1;
                        match chars[i] {
                            'n' => push(&mut units, '\n'),
                            't' => push(&mut units, '\t'),
                            'r' => push(&mut units, '\r'),
                            'u' if i + 4 < chars.len() => {
                                let hex: String = chars[i + 1..i + 5].iter().collect();
                                i += 4;
                                match i32::from_str_radix(&hex, 16) {
                                    Ok(n) => units.push(n as u16),
                                    Err(_) => {
                                        out.push(Word { kind: Kind::LexError, text: format!("Invalid unicode escape: \\u{}", hex), start, end: start + 1 });
                                        push(&mut units, ' ');
                                    }
                                }
                            }
                            other => push(&mut units, other),
                        }
                        i += 1;
                    }
                    other => {
                        push(&mut units, other);
                        i += 1;
                    }
                }
            }
            let word = String::from_utf16_lossy(&units);
            if closed {
                out.push(Word { kind: Kind::StringLit, text: word, start, end: i });
            } else {
                out.push(Word { kind: Kind::LexError, text: "Unterminated string literal".to_string(), start, end: i });
            }
        } else if chars[i] == '`' {
            // A backtick-quoted identifier is its text, blanks included; one unterminated (the rest of the line
            // its) or empty is the lexer's error.
            match chars[i + 1..].iter().position(|&c| c == '`') {
                Some(n) if n > 0 => {
                    out.push(Word { kind: Kind::Ident, text: chars[i + 1..i + 1 + n].iter().collect(), start, end: i + n + 2 });
                    i += n + 2;
                }
                Some(_) => {
                    i += 2;
                    out.push(Word { kind: Kind::LexError, text: "Empty backtick identifier".to_string(), start, end: i });
                }
                None => {
                    i = chars.len();
                    out.push(Word { kind: Kind::LexError, text: "Unterminated backtick identifier".to_string(), start, end: i });
                }
            }
        } else {
            let mut word = String::new();
            while i < chars.len() && !blank(chars[i]) && !separator(i) {
                if chars[i] == '"' {
                    out.push(Word {
                        kind: Kind::LexError,
                        text: "Whitespace is required between values — a quote cannot immediately follow another value.".to_string(),
                        start: i,
                        end: i + 1,
                    });
                    break;
                }
                word.push(chars[i]);
                i += 1;
            }
            if !word.is_empty() {
                let kind = match word.as_str() {
                    "using" => Kind::Using,
                    "true" | "false" => Kind::BoolLit,
                    _ => Kind::Ident,
                };
                out.push(Word { kind, text: word, start, end: i });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The texts of a directive's words.
    fn words(text: &str) -> Vec<String> {
        values(text).into_iter().filter(|w| w.kind != Kind::Comma).map(|w| w.text).collect()
    }

    /// The kinds and texts of a directive's tokens.
    fn tokens(text: &str) -> Vec<(Kind, String)> {
        values(text).into_iter().map(|w| (w.kind, w.text)).collect()
    }

    #[test]
    fn a_header_is_its_first_line_or_runs_to_its_closer() {
        assert_eq!(script_header("object A"), 0);
        assert_eq!(script_header("#!/usr/bin/env -S teq interp\nobject A"), 28);
        assert_eq!(script_header("#!/bin/sh\nexec scala \"$0\"\n!#\nobject A"), 29);
        assert_eq!(script_header("::#!\n@echo off\n::!#\nobject A"), 20);
        assert_eq!(script_header("#!x\r\nobject A"), 4);
        assert_eq!(script_header("#!only"), 6);
    }

    #[test]
    fn directives_of_the_header_alone() {
        let text = "#!/usr/bin/env -S teq interp\n// a comment\n/* a\n block */\n//> using scala 3.8.4\n//> using file ../lib\n//> using files \"a b.scala\", c.scala\nobject A\n//> using file late.scala\n";
        let found = using_files(text);
        assert_eq!(found.len(), 2);
        assert_eq!((found[0].line, found[0].paths.clone()), (6, vec!["../lib".to_string()]));
        assert_eq!((found[1].line, found[1].paths.clone()), (7, vec!["a b.scala".to_string(), "c.scala".to_string()]));
    }

    #[test]
    fn the_source_of_the_options() {
        let text = "//> using scala 3.8.4\n//> using options -deprecation -source:3.3\n//> using option -source 3.0-migration\nobject A\n//> using options -source:future\n";
        assert_eq!(using_source(text), Some((3, "3.0-migration".to_string())));
        assert_eq!(using_source("//> using scalacOptions -source:3.4\nobject A"), Some((1, "3.4".to_string())));
        assert_eq!(using_source("//> using options -deprecation\nobject A"), None);
    }

    #[test]
    fn a_directive_starts_with_the_exact_prefix_its_words_apart_by_blanks() {
        let deps = |text: &str| using_deps(text).into_iter().map(|d| (d.line, d.coordinates)).collect::<Vec<_>>();
        assert_eq!(deps("//> using\tdep a:b:1\nobject A\n"), vec![(1, vec!["a:b:1".to_string()])]);
        assert_eq!(deps("//> using dep\ta:b:1\x0cc:d:2\nobject A\n"), vec![(1, vec!["a:b:1".to_string(), "c:d:2".to_string()])]);
        assert_eq!(deps("  \t//> using dep a:b:1\n"), vec![(1, vec!["a:b:1".to_string()])]);
        // `//>using` and `//>\tusing` are no directives: the header ends, the first at it, the second after it.
        assert!(deps("//>using dep a:b:1\n//> using dep c:d:2\n").is_empty());
        assert!(deps("//>\tusing dep a:b:1\n//> using dep c:d:2\n").is_empty());
        assert_eq!(deps("//> using dep a:b:1\n//>  using dep c:d:2\n//> using dep e:f:3\n"), vec![(1, vec!["a:b:1".to_string()])]);
    }

    #[test]
    fn a_backtick_quoted_word_is_its_text() {
        let deps = |text: &str| using_deps(text).into_iter().map(|d| d.coordinates).collect::<Vec<_>>();
        assert_eq!(deps("//> using `dep` dev.zio::zio:2.1.25\n"), vec![vec!["dev.zio::zio:2.1.25".to_string()]]);
        assert_eq!(deps("//> using dep `a:b:1`\n"), vec![vec!["a:b:1".to_string()]]);
        assert_eq!(words("using `a b` c"), vec!["using", "a b", "c"]);
        // An empty and an unterminated one are the lexer's errors, the second the rest of the line.
        assert_eq!(tokens("using `` c"), vec![(Kind::Using, "using".to_string()), (Kind::LexError, "Empty backtick identifier".to_string()), (Kind::Ident, "c".to_string())]);
        assert_eq!(tokens("using `dep c"), vec![(Kind::Using, "using".to_string()), (Kind::LexError, "Unterminated backtick identifier".to_string())]);
    }

    #[test]
    fn a_comma_separates_before_white_space_alone() {
        assert_eq!(words("dep org.scala-lang:scala-library:3.8.4,classifier=sources"), vec!["dep", "org.scala-lang:scala-library:3.8.4,classifier=sources"]);
        assert_eq!(words("files a.scala, b.scala,"), vec!["files", "a.scala", "b.scala"]);
        assert_eq!(words("files a.scala,b.scala"), vec!["files", "a.scala,b.scala"]);
        assert_eq!(words("dep \"a:b:1,classifier=x\", c:d:2"), vec!["dep", "a:b:1,classifier=x", "c:d:2"]);
    }

    #[test]
    fn a_nested_comment_holds_its_directives() {
        let text = "/* outer\n /* inner */\n//> using file hidden.scala\n*/\n//> using file seen.scala\nobject A\n";
        let found = using_files(text);
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].line, found[0].paths.clone()), (5, vec!["seen.scala".to_string()]));
        // An unterminated comment holds the rest.
        assert!(using_files("/* /* */\n//> using file x.scala\n").is_empty());
    }

    #[test]
    fn a_quoted_word_is_decoded_as_scala_clis_lexer_decodes_it() {
        assert_eq!(words(r#"dep "dev.zio::zio:\u0032.1.26""#), vec!["dep", "dev.zio::zio:2.1.26"]);
        assert_eq!(words(r#"dep "a\tb\nc\rd\\e\"f""#), vec!["dep", "a\tb\nc\rd\\e\"f"]);
        // Any other escaped character is itself; a `\u` without four characters and a quote after it is `u`.
        assert_eq!(words(r#"dep "x\x32""#), vec!["dep", "xx32"]);
        assert_eq!(words(r#"dep "2.1.2\u""#), vec!["dep", "2.1.2u"]);
        // `Integer.parseInt` takes a sign before the digits.
        assert_eq!(words(r#"dep "\u+041z""#), vec!["dep", "Az"]);
        // A backslash that ends the line stays, and the string is unterminated.
        let w = values(r#"dep "a\"#);
        assert_eq!((w[1].kind, w[1].text.as_str(), w[1].start), (Kind::LexError, "Unterminated string literal", 4));
    }

    #[test]
    fn the_lexers_errors_and_a_deprecated_keys_warning_stand_where_scala_cli_reports_them() {
        let at = |text: &str| problems(text).into_iter().map(|p| (p.error, p.line, p.col, p.width, p.msg)).collect::<Vec<_>>();
        assert_eq!(at("//> using dep \"dev.zio::zio:\\u00zz.1.26\"\n"), vec![(true, 1, 15, 1, "Lexer error: Invalid unicode escape: \\u00zz".to_string())]);
        assert_eq!(at("// x\n//> using dep \"dev.zio::zio:2.1.26\n"), vec![(true, 2, 15, 1, "Lexer error: Unterminated string literal".to_string())]);
        assert_eq!(at("//> using dep a\"b\"\n").len(), 1);
        assert_eq!(at("//> using lib dev.zio::zio:2.1.25\n"), vec![(false, 1, 11, 23, "Using 'lib' is deprecated, use 'dep' instead".to_string())]);
        assert_eq!(at("  //> using libs a:b:1 c:d:2\n"), vec![(false, 1, 13, 16, "Using 'libs' is deprecated, use 'deps' instead".to_string())]);
        assert_eq!(at("//> using compileOnly.lib a:b:1\n")[0].4, "Using 'compileOnly.lib' is deprecated, use 'compileOnly.dep' instead");
        assert!(at("//> using dep a:b:1\n//> using test.dep c:d:2\n").is_empty());
    }

    #[test]
    fn every_key_of_the_dependencies_a_run_holds_is_a_dependency() {
        let keys = |text: &str| using_deps(text).into_iter().map(|d| d.key).collect::<Vec<_>>();
        for key in ["dep", "deps", "dependency", "dependencies", "lib", "libs", "compileOnly.dep", "compileOnly.deps", "compileOnly.dependencies", "compileOnly.lib", "compileOnly.libs", "compileOnlyDependency", "compile-only.dep", "compile-only-dependency"] {
            assert_eq!(keys(&format!("//> using {} a:b:1\n", key)), vec![key.to_string()], "{}", key);
        }
        for key in ["test.dep", "test.dependency", "scalafix.dep", "compileonly.dep", "Dep"] {
            assert!(keys(&format!("//> using {} a:b:1\n", key)).is_empty(), "{}", key);
        }
    }

    #[test]
    fn a_key_that_is_no_identifier_is_the_parsers_error() {
        let at = |text: &str| problems(text).into_iter().map(|p| (p.error, p.col, p.msg)).collect::<Vec<_>>();
        let found = |kind: &str| format!("Expected a key after `using`, found: {}", kind);
        assert_eq!(at("//> using `` dev.zio::zio:2.1.25\n"), vec![(true, 11, found("LexError"))]);
        assert_eq!(at("//> using `dep dev.zio::zio:2.1.25\n"), vec![(true, 11, found("LexError"))]);
        assert_eq!(at("//> using \"dep\" \"a:b:1\"\n"), vec![(true, 11, found("StringLit"))]);
        assert_eq!(at("//> using true a:b:1\n"), vec![(true, 11, found("BoolLit"))]);
        assert_eq!(at("//> using\n"), vec![(true, 5, "Expected a key after `using`".to_string())]);
        assert_eq!(at("//> usingx a:b:1\n"), vec![(true, 5, "Unexpected token: Ident".to_string())]);
        // None of them is a directive.
        assert!(using_deps("//> using `` a:b:1\n//> using \"dep\" c:d:2\n").is_empty());
    }

    #[test]
    fn a_block_comments_closing_line_with_more_on_it_starts_the_code() {
        let text = "/* header */ // ordinary comment\n//> using dep a:b:1\nprintln(\"ok\")\n";
        assert!(using_deps(text).is_empty());
        let warned = problems(text).into_iter().map(|p| (p.error, p.line, p.col, p.msg)).collect::<Vec<_>>();
        assert_eq!(warned, vec![(false, 2, 1, "Ignoring using directive found after Scala code: //> using dep a:b:1".to_string())]);
        // A block comment alone on its lines is skipped, the directives after it read.
        assert_eq!(using_deps("/* one\n two */  \n//> using dep a:b:1\n").len(), 1);
        // A misspelt prefix is warned of, and the header ends after its line.
        let text = "//>  using dep a:b:1\n//> using dep c:d:2\nobject A\n";
        assert!(using_deps(text).is_empty());
        let warned = problems(text).into_iter().map(|p| (p.line, p.msg)).collect::<Vec<_>>();
        assert_eq!(warned[0], (1, "Using directive must use the exact prefix `//> using`. Invalid prefix in: //>  using dep a:b:1".to_string()));
        assert_eq!(warned[1], (2, "Ignoring using directive found after Scala code: //> using dep c:d:2".to_string()));
    }
}
