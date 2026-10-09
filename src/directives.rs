//! What a source file says before its code for the tools around the compiler: a script header
//! (`#!/usr/bin/env -S teq interp`), which scalac skips, and Scala CLI's `//> using` directives, of
//! which `teq interp` honours `file` and `files` (the sources a script includes) and leaves the
//! others as the comments they are to scalac.

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

/// The `file` and `files` directives among the directives that open `text`, as Scala CLI reads
/// them: after the script header, the lines before the first code (blank lines and comments, a
/// directive being one) are the header, and a directive past the first code is ignored. A value
/// is a word or a double-quoted string (`\"` and `\\` escaped), values separated by white space or,
/// as Scala CLI still accepts, commas.
pub fn using_files(text: &str) -> Vec<UsingFiles> {
    let mut out = Vec::new();
    let bytes = text.as_bytes();
    let mut pos = script_header(text);
    loop {
        while pos < bytes.len() && matches!(bytes[pos], b' ' | b'\t' | b'\r' | b'\n') {
            pos += 1;
        }
        let rest = &text[pos..];
        if rest.starts_with("/*") {
            // A block comment, nested ones in it, by the lexer's rule; code after it is code.
            match crate::lexer::block_comment_end(bytes, pos) {
                Some(end) => pos = end,
                None => break,
            }
            continue;
        }
        let Some(comment) = rest.strip_prefix("//") else { break };
        let line_end = comment.find('\n').map_or(text.len(), |i| pos + 2 + i);
        let comment = text[pos + 2..line_end].strip_suffix('\r').unwrap_or(&text[pos + 2..line_end]);
        if let Some(directive) = comment.strip_prefix("> using ").or_else(|| comment.strip_prefix(">using ")) {
            let words = values(directive);
            if let Some((key, paths)) = words.split_first() {
                if (key == "file" || key == "files") && !paths.is_empty() {
                    out.push(UsingFiles { line: text[..pos].matches('\n').count() + 1, paths: paths.to_vec() });
                }
            }
        }
        pos = line_end;
    }
    out
}

/// The words of a directive: its key, then its values.
fn values(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut chars = text.chars().peekable();
    loop {
        while chars.peek().is_some_and(|&c| c.is_whitespace() || c == ',') {
            chars.next();
        }
        let Some(&first) = chars.peek() else { break };
        let mut word = String::new();
        if first == '"' {
            chars.next();
            while let Some(c) = chars.next() {
                match c {
                    '"' => break,
                    '\\' => word.extend(chars.next()),
                    c => word.push(c),
                }
            }
        } else {
            while let Some(&c) = chars.peek() {
                if c.is_whitespace() || c == ',' {
                    break;
                }
                word.push(c);
                chars.next();
            }
        }
        out.push(word);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn a_nested_comment_holds_its_directives() {
        let text = "/* outer\n /* inner */\n//> using file hidden.scala\n*/\n//> using file seen.scala\nobject A\n";
        let found = using_files(text);
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].line, found[0].paths.clone()), (5, vec!["seen.scala".to_string()]));
        // An unterminated comment holds the rest.
        assert!(using_files("/* /* */\n//> using file x.scala\n").is_empty());
    }
}
