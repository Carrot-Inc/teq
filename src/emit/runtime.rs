//! The runtime split into its top-level definitions, so that the output carries the helpers the
//! program refers to and what those refer to in turn. A definition starts at a line that begins in
//! the first column (`function`, `const`, `let`, `class`) and takes the comment lines before it;
//! statements at the top level (`"use strict"`, the `if`s that set up printing) are always kept.

use crate::intern::FxMap;

pub struct Runtime {
    items: Vec<Item>,
    by_name: FxMap<&'static str, usize>,
    release: bool,
}

struct Item {
    text: &'static str,
    name: Option<&'static str>,
    /// The other items this one refers to.
    deps: Vec<usize>,
}

pub type Used = Vec<bool>;

impl Runtime {
    /// Splits the runtime into items; `release` writes them without comments and indentation.
    pub fn parse(src: &'static str, release: bool) -> Runtime {
        let mut items: Vec<Item> = Vec::new();
        let mut by_name: FxMap<&'static str, usize> = FxMap::default();
        let mut start = 0;
        let mut comment: Option<usize> = None;
        let mut pos = 0;
        let mut first = true;
        for line in src.split_inclusive('\n') {
            let at = pos;
            pos += line.len();
            let head = line.as_bytes().first().copied().unwrap_or(b'\n');
            if line.starts_with("//") {
                comment.get_or_insert(at);
                continue;
            }
            if head == b' ' || head == b'}' || head == b')' || head == b'\n' {
                comment = None;
                continue;
            }
            let item_start = comment.take().unwrap_or(at);
            if !first {
                Self::push(&mut items, &mut by_name, &src[start..item_start]);
            }
            first = false;
            start = item_start;
        }
        Self::push(&mut items, &mut by_name, &src[start..]);
        for i in 0..items.len() {
            let mut deps = Vec::new();
            for_each_helper(items[i].text, |name| {
                if let Some(&j) = by_name.get(name) {
                    if j != i && !deps.contains(&j) {
                        deps.push(j);
                    }
                }
            });
            items[i].deps = deps;
        }
        Runtime { items, by_name, release }
    }

    fn push(items: &mut Vec<Item>, by_name: &mut FxMap<&'static str, usize>, text: &'static str) {
        let code = text.lines().find(|l| !l.starts_with("//")).unwrap_or("");
        let name = ["function ", "const ", "let ", "class "]
            .iter()
            .find_map(|kw| code.strip_prefix(kw))
            .map(|rest| &rest[..rest.bytes().position(|b| !is_ident(b)).unwrap_or(rest.len())])
            .filter(|n| n.starts_with('$'));
        if let Some(n) = name {
            by_name.insert(n, items.len());
        }
        items.push(Item { text, name, deps: Vec::new() });
    }

    pub fn none_used(&self) -> Used {
        vec![false; self.items.len()]
    }

    /// Marks the items that `text` refers to.
    pub fn mark(&self, text: &str, used: &mut Used) {
        for_each_helper(text, |name| {
            if let Some(&i) = self.by_name.get(name) {
                used[i] = true;
            }
        });
    }

    pub fn merge(into: &mut Used, from: &Used) {
        for (a, b) in into.iter_mut().zip(from) {
            *a |= *b;
        }
    }

    /// The names of the used items, in their original order.
    pub fn names(&self, used: &Used) -> Vec<&'static str> {
        self.items.iter().enumerate().filter(|&(i, _)| used[i]).filter_map(|(_, it)| it.name).collect()
    }

    /// Writes the unnamed items, the used ones and what those depend on, in their original order.
    pub fn write(&self, used: &Used, out: &mut String) {
        let mut keep: Vec<bool> = self.items.iter().enumerate().map(|(i, it)| it.name.is_none() || used[i]).collect();
        let mut stack: Vec<usize> = (0..self.items.len()).filter(|&i| keep[i]).collect();
        while let Some(i) = stack.pop() {
            for &d in &self.items[i].deps {
                if !keep[d] {
                    keep[d] = true;
                    stack.push(d);
                }
            }
        }
        for (i, item) in self.items.iter().enumerate() {
            if keep[i] {
                if self.release {
                    strip(item.text, out);
                } else {
                    out.push_str(item.text);
                }
            }
        }
    }

    pub fn helper_names(&self) -> Vec<&'static str> {
        self.items.iter().filter_map(|it| it.name).collect()
    }

    pub fn len(&self) -> usize {
        self.items.iter().map(|i| i.text.len()).sum()
    }
}

/// Appends the item without its comment lines and indentation.
fn strip(text: &str, out: &mut String) {
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.is_empty() {
            continue;
        }
        out.push_str(trimmed);
        out.push('\n');
    }
}

fn is_ident(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

/// Calls `f` with every `$name` token of `text` that starts an identifier.
fn for_each_helper(text: &str, mut f: impl FnMut(&str)) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while let Some(at) = next_dollar(bytes, i) {
        let mut end = at + 1;
        while end < bytes.len() && is_ident(bytes[end]) {
            end += 1;
        }
        if at == 0 || !is_ident(bytes[at - 1]) {
            f(&text[at..end]);
        }
        i = end;
    }
}

/// The next `$` at or after `from`, eight bytes at a time.
fn next_dollar(bytes: &[u8], mut from: usize) -> Option<usize> {
    const DOLLARS: u64 = 0x2424_2424_2424_2424;
    const LOW: u64 = 0x0101_0101_0101_0101;
    const HIGH: u64 = 0x8080_8080_8080_8080;
    while from + 8 <= bytes.len() {
        let word = u64::from_le_bytes(bytes[from..from + 8].try_into().unwrap()) ^ DOLLARS;
        let zero = word.wrapping_sub(LOW) & !word & HIGH;
        if zero != 0 {
            return Some(from + (zero.trailing_zeros() / 8) as usize);
        }
        from += 8;
    }
    bytes[from..].iter().position(|&b| b == b'$').map(|p| from + p)
}
