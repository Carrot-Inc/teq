//! A library's compile options as the build takes them: each a token of a form of the table
//! `library-options.txt`, kept as it is or, a dependency output's option, left out (the build's own write the list
//! it reads, build.rs); any other option is refused by name. Every form the table keeps is one self-contained
//! token: zig 0.16.0's option table (its src/clang_options.zon), searched as zig searches it, takes no token after
//! it (tests/native-build/library-options.scala checks the table against zig's and writes the cases the suite
//! builds).

use std::sync::OnceLock;

pub const TABLE: &str = include_str!("library-options.txt");

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Role {
    /// Given to zig as it is.
    Kept,
    /// A dependency output's option, its value in the token: left out.
    Dropped,
    /// A dependency output's option whose value is the token after it: both left out.
    DroppedWithNext,
}

/// A form of the table: a token as it is written, or a prefix and a value that is not empty and is none of the
/// values the form excepts.
#[derive(Debug)]
pub struct Form {
    pub spelling: &'static str,
    pub prefix: bool,
    pub role: Role,
    /// The values the form does not take: written whole, or (`true`) by their start.
    pub except: Vec<(&'static str, bool)>,
}

impl Form {
    /// Whether a token is of the form.
    pub fn takes(&self, token: &str) -> bool {
        if !self.prefix {
            return token == self.spelling;
        }
        let Some(value) = token.strip_prefix(self.spelling) else { return false };
        !value.is_empty() && !self.except.iter().any(|&(except, start)| if start { value.starts_with(except) } else { value == except })
    }
}

/// A form written `<spelling>*` takes a value; `!<value>` and `!<start>*` are the values it does not take.
fn star(word: &'static str) -> (&'static str, bool) {
    match word.strip_suffix('*') {
        Some(start) => (start, true),
        None => (word, false),
    }
}

/// The table's forms, in its order.
pub fn forms() -> &'static [Form] {
    static FORMS: OnceLock<Vec<Form>> = OnceLock::new();
    FORMS.get_or_init(|| {
        TABLE
            .lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(|line| {
                let fields: Vec<&'static str> = line.split_whitespace().collect();
                let (spelling, prefix) = star(fields[0]);
                let role = match fields[1] {
                    "kept" => Role::Kept,
                    "dropped" => Role::Dropped,
                    "dropped+next" => Role::DroppedWithNext,
                    other => panic!("library-options.txt: no role {}", other),
                };
                let except = fields.iter().filter_map(|field| field.strip_prefix('!')).map(star).collect();
                Form { spelling, prefix, role, except }
            })
            .collect()
    })
}

/// The form a token is of: the one written as the token, else the one of the longest spelling that takes it.
pub fn form_of(token: &str) -> Option<&'static Form> {
    let forms = forms();
    forms.iter().find(|form| !form.prefix && form.spelling == token).or_else(|| forms.iter().filter(|form| form.prefix && form.takes(token)).max_by_key(|form| form.spelling.len()))
}

/// The options as zig is to be given them, or the first that the build does not take.
pub fn taken(given: &[String]) -> Result<Vec<String>, String> {
    let mut kept = Vec::new();
    let mut at = 0;
    while at < given.len() {
        let token = &given[at];
        match form_of(token).map(|form| form.role) {
            Some(Role::Kept) => kept.push(token.clone()),
            Some(Role::Dropped) => {}
            Some(Role::DroppedWithNext) if at + 1 < given.len() => at += 1,
            _ => return Err(token.clone()),
        }
        at += 1;
    }
    Ok(kept)
}

/// What a refusal of a library's option says (build.rs gives the library and the descriptor).
pub fn refusal(option: &str) -> String {
    if option.is_empty() {
        "an empty compile option in a library's descriptor".to_string()
    } else {
        format!("the compile option {} in a library's descriptor", option)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    /// Every case of the suite's table (tests/native-build/library-options.expected) as the classifier answers it,
    /// after its self-test: planted bad cases, each of which the verdict must find.
    #[test]
    fn takes_the_tables_forms_and_refuses_the_rest_by_name() {
        let verdict = |cases: &str| -> Vec<String> {
            let mut faults = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for line in cases.lines().filter(|line| !line.is_empty() && !line.starts_with('#')) {
                // An answer, a language and the options, empty where the line ends after the language.
                let fields: Vec<&str> = line.splitn(3, ' ').collect();
                let (expected, tokens) = match fields[..] {
                    [expected, _] | [expected, _, ""] => (expected, ""),
                    [expected, _, tokens] => (expected, tokens),
                    _ => {
                        faults.push(format!("{line}: not an answer, a language and options"));
                        continue;
                    }
                };
                if !seen.insert(line) {
                    faults.push(format!("{line}: twice"));
                }
                let given: Vec<String> = tokens.split(',').map(String::from).collect();
                let answer = match taken(&given) {
                    Ok(kept) if kept == given => "passed".to_string(),
                    Ok(kept) if kept.is_empty() => "dropped".to_string(),
                    Ok(kept) => format!("kept {}", kept.join(",")),
                    Err(option) if option == given[0] => "refused".to_string(),
                    Err(option) => format!("refused at {option}"),
                };
                if answer != expected {
                    faults.push(format!("{tokens}: {answer}, not {expected}"));
                }
            }
            for kind in ["passed", "dropped", "refused"] {
                if !cases.lines().any(|line| line.starts_with(&format!("{kind} "))) {
                    faults.push(format!("no case {kind}"));
                }
            }
            faults
        };
        let good = "passed c -std=gnu11\ndropped c -MD\nrefused c -Xclang\n";
        let planted = [
            ("a case answered wrong", "passed c -MD\ndropped c -std=gnu11\nrefused c -Xclang\n"),
            ("a case twice", "passed c -std=gnu11\npassed c -std=gnu11\ndropped c -MD\nrefused c -Xclang\n"),
            ("an empty table", ""),
            ("a table cut short", "passed c -std=gnu11\n"),
            ("a case without a language", "passed\ndropped c -MD\nrefused c -Xclang\n"),
            ("a refusal of another option than the first", "passed c -std=gnu11\ndropped c -MD\nrefused c -DX,-Xclang\n"),
        ];
        for (what, cases) in planted {
            assert!(!verdict(cases).is_empty(), "the self-test: {what} was taken");
        }
        assert!(verdict(good).is_empty(), "the self-test: the good table was refused");
        let cases = include_str!("../../tests/native-build/library-options.expected");
        let found = verdict(cases);
        assert!(found.is_empty(), "{} faults:\n{}", found.len(), found.join("\n"));
        // Of every form, a case of its answer: a kept form's passed, a dropped form's dropped.
        let coverage = |cases: &str| -> Vec<String> {
            let answered: Vec<(&str, &str)> = cases.lines().filter(|line| !line.is_empty() && !line.starts_with('#')).map(|line| {
                let fields: Vec<&str> = line.splitn(3, ' ').collect();
                (fields[0], fields.get(2).map_or("", |options| options.split(',').next().unwrap_or("")))
            }).collect();
            forms().iter().filter(|form| {
                let wanted = if form.role == Role::Kept { "passed" } else { "dropped" };
                !answered.iter().any(|&(answer, first)| answer == wanted && form.takes(first))
            }).map(|form| format!("{}: no case of the form", form.spelling)).collect()
        };
        let without_std: String = cases.lines().filter(|line| !line.starts_with("passed c -std=") && !line.starts_with("passed cpp -std=")).map(|line| format!("{line}\n")).collect();
        assert!(!coverage(&without_std).is_empty(), "the self-test: a form without a case was taken");
        let uncovered = coverage(cases);
        assert!(uncovered.is_empty(), "{}", uncovered.join("\n"));
    }

    #[test]
    fn leaves_out_a_dependency_option_with_its_value_alone() {
        assert_eq!(taken(&strings(&["-DA", "-MF", "-DB", "-DC"])), Ok(strings(&["-DA", "-DC"])));
        assert_eq!(taken(&strings(&["-DA", "-MF"])), Err("-MF".to_string()));
        assert_eq!(taken(&strings(&["-I", "-MD", "-DFOO=1"])), Err("-I".to_string()));
        assert_eq!(taken(&strings(&["-DA", ""])), Err(String::new()));
        assert_eq!(refusal(""), "an empty compile option in a library's descriptor");
    }
}
