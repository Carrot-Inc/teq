//! A directory as the bytes of its files, by relative path.

use std::collections::BTreeMap;
use std::path::Path;

pub type Tree = BTreeMap<String, Vec<u8>>;

pub fn read(dir: &Path) -> Tree {
    let mut tree = Tree::new();
    if dir.is_dir() {
        walk(dir, dir, &mut tree);
    }
    tree
}

fn walk(root: &Path, dir: &Path, tree: &mut Tree) {
    let entries = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot list {}: {e}", dir.display()));
    for entry in entries {
        let path = entry.unwrap().path();
        if path.is_dir() {
            walk(root, &path, tree);
        } else {
            let name = path.strip_prefix(root).unwrap().to_string_lossy().into_owned();
            let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
            tree.insert(name, bytes);
        }
    }
}

/// The names of the files that differ, each with the first line that does.
pub fn differences(left: &Tree, right: &Tree, left_name: &str, right_name: &str) -> Vec<String> {
    let mut found = Vec::new();
    for name in left.keys().chain(right.keys().filter(|name| !left.contains_key(*name))) {
        match (left.get(name), right.get(name)) {
            (Some(a), Some(b)) if a == b => {}
            (Some(a), Some(b)) => {
                let a = String::from_utf8_lossy(a);
                let b = String::from_utf8_lossy(b);
                let line = a.lines().zip(b.lines()).position(|(x, y)| x != y);
                let at = line.unwrap_or_else(|| a.lines().count().min(b.lines().count()));
                found.push(format!(
                    "{name} differs at line {}:\n    {left_name}: {}\n    {right_name}: {}",
                    at + 1,
                    a.lines().nth(at).unwrap_or("<the file ends>"),
                    b.lines().nth(at).unwrap_or("<the file ends>"),
                ));
            }
            (Some(_), None) => found.push(format!("{name} is in the {left_name} only")),
            (None, _) => found.push(format!("{name} is in the {right_name} only")),
        }
    }
    found
}
