//! The hash tables Scala Native 0.5.12's link keeps its state in, and the hashes of what it keys
//! them by: `mutable.HashSet` and `mutable.HashMap` of scala-library 3.8.4, the library its tools
//! run on in the oracle (the tables of 2.13's, written again for Scala 3), and the `hashCode` of
//! a signature, a name, a path and a position as the JVM computes it. Where Scala Native's link
//! iterates such a table its order decides what is reached first, and so the order a class's
//! signatures were called in, which numbers its virtual table where two of them share a hash:
//! teq's link keeps the same tables, operation for operation, and iterates them in their order.
//!
//! The order of a table is its buckets' (an element's improved hash, `h ^ (h >>> 16)`, cut to the
//! table's length at the moment of iteration), within a bucket the improved hashes' (as signed
//! integers), and of one hash the order the elements were added in (`addElem` and `put0` put an
//! element after those whose hash is not above its own, `growTable` keeps each bucket's order). The
//! length starts at 16 and doubles before an addition that would reach three quarters of it,
//! present or not; `++=` first grows it to hold the other table (`sizeHint`), a set then adding
//! without that check and a map with it; a removal or `clear` never shrinks it. An immutable map of
//! more than four entries is a trie of its keys' improved hashes (`immutable_order`), whose order
//! the keys alone decide.

use crate::intern::FxMap;
use std::hash::Hash;

const LOAD_FACTOR: f64 = 0.75;

/// `java.lang.String.hashCode` of a string given in NIR's bytes.
pub fn string_hash(bytes: &[u8]) -> i32 {
    if bytes.is_ascii() {
        bytes.iter().fold(0i32, |h, &b| h.wrapping_mul(31).wrapping_add(b as i32))
    } else {
        super::nir::utf16_units(bytes).fold(0i32, |h, u| h.wrapping_mul(31).wrapping_add(u as i32))
    }
}

fn mix_last(hash: i32, data: i32) -> i32 {
    let k = data.wrapping_mul(0xcc9e2d51u32 as i32).rotate_left(15).wrapping_mul(0x1b873593);
    hash ^ k
}

fn mix(hash: i32, data: i32) -> i32 {
    mix_last(hash, data).rotate_left(13).wrapping_mul(5).wrapping_add(0xe6546b64u32 as i32)
}

fn finalize_hash(hash: i32, length: i32) -> i32 {
    let mut h = (hash ^ length) as u32;
    h ^= h >> 16;
    h = h.wrapping_mul(0x85ebca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2ae35);
    h ^= h >> 16;
    h as i32
}

/// A case class's synthetic `hashCode` (`MurmurHash3.caseClassHash`): its name's hash and its
/// fields' hashes, mixed from `productSeed`.
pub fn case_class_hash(name: &str, fields: &[i32]) -> i32 {
    let prefix = string_hash(name.as_bytes());
    if fields.is_empty() {
        return prefix;
    }
    let mut h = mix(0xcafebabeu32 as i32, prefix);
    for &field in fields {
        h = mix(h, field);
    }
    finalize_hash(h, fields.len() as i32)
}

/// `nir.Global.Top(id)`.
pub fn top_hash(id: &[u8]) -> i32 {
    case_class_hash("Top", &[string_hash(id)])
}

/// `nir.Global.Member(owner, sig)`, `owner` the hash of its `Top` and `sig` of its mangled name
/// (`Sig.hashCode` is its `mangle`'s).
pub fn member_hash(owner: i32, sig: i32) -> i32 {
    case_class_hash("Member", &[owner, sig])
}

/// A path's spelling as `Paths.get` keeps it: no empty name, no trailing slash but the root's.
fn normal_path(path: &[u8]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::with_capacity(path.len());
    for &b in path {
        if !(b == b'/' && out.last() == Some(&b'/')) {
            out.push(b);
        }
    }
    if out.len() > 1 && out.last() == Some(&b'/') {
        out.pop();
    }
    out
}

/// `sun.nio.fs.UnixPath.hashCode`: of a path of the default file system (a directory of the class
/// path, a file under it).
pub fn unix_path_hash(path: &[u8]) -> i32 {
    normal_path(path).iter().fold(0i32, |h, &b| h.wrapping_mul(31).wrapping_add(b as i32))
}

/// `jdk.nio.zipfs.ZipPath.hashCode` (`Arrays.hashCode` of its bytes): of a file in a jar.
pub fn zip_path_hash(path: &[u8]) -> i32 {
    normal_path(path).iter().fold(1i32, |h, &b| h.wrapping_mul(31).wrapping_add(b as i8 as i32))
}

/// `nir.SourcePosition(source, line, column, nirSource)`: `file` the source's path (empty for
/// `SourceFile.Virtual`), `nir_source` the hashes of the class path entry's path and the NIR
/// file's (none for `NIRSource.None`).
pub fn position_hash(file: &[u8], line: u32, column: u32, nir_source: Option<(i32, i32)>) -> i32 {
    let source = if file.is_empty() { string_hash(b"Virtual") } else { case_class_hash("Relative", &[string_hash(file)]) };
    let (directory, path) = nir_source.unwrap_or((0, 0));
    case_class_hash("SourcePosition", &[source, line as i32, column as i32, case_class_hash("NIRSource", &[directory, path])])
}

/// `nir.Type.Ref(Global.Top(name))`: not exact, nullable (`Boolean.hashCode`: 1237, 1231).
pub fn ref_type_hash(name: &[u8]) -> i32 {
    case_class_hash("Ref", &[top_hash(name), 1237, 1231])
}

/// A case object's (`nir.Type.Bool` and the like): its name's hash.
pub fn case_object_hash(name: &str) -> i32 {
    string_hash(name.as_bytes())
}

/// The largest immutable map scala-library keeps in the order of its entries: `Map1` to `Map4`
/// (`MapBuilderImpl.addOne` switches to a `HashMap` at the fifth key).
const SMALL_MAP: usize = 4;

/// The order of the keys of an immutable map built from entries in the order of `hashes` (their
/// `hashCode`s): a small map's in that order (`SMALL_MAP`), a larger one's in the trie's (CHAMP, by
/// `Hashing.improve` of each hash, five bits a level from the lowest): a node's own entries by
/// their bits, then each of its nodes by theirs, the deepest of one hash in their order.
pub fn immutable_order(hashes: &[i32]) -> Vec<usize> {
    fn improve(h: i32) -> u32 {
        let mut h = h.wrapping_add(!(h << 9));
        h ^= ((h as u32) >> 14) as i32;
        h = h.wrapping_add(h << 4);
        (h ^ ((h as u32) >> 10) as i32) as u32
    }
    fn node(keys: &[(u32, usize)], shift: u32, out: &mut Vec<usize>) {
        if shift >= 32 {
            out.extend(keys.iter().map(|k| k.1));
            return;
        }
        let mut chunks: Vec<(u32, Vec<(u32, usize)>)> = Vec::new();
        for &k in keys {
            let chunk = (k.0 >> shift) & 31;
            match chunks.iter_mut().find(|c| c.0 == chunk) {
                Some(c) => c.1.push(k),
                None => chunks.push((chunk, vec![k])),
            }
        }
        chunks.sort_by_key(|c| c.0);
        out.extend(chunks.iter().filter(|c| c.1.len() == 1).map(|c| c.1[0].1));
        for (_, group) in chunks.iter().filter(|c| c.1.len() > 1) {
            node(group, shift + 5, out);
        }
    }
    if hashes.len() <= SMALL_MAP {
        return (0..hashes.len()).collect();
    }
    let keys: Vec<(u32, usize)> = hashes.iter().enumerate().map(|(i, &h)| (improve(h), i)).collect();
    let mut out = Vec::with_capacity(keys.len());
    node(&keys, 0, &mut out);
    out
}

/// `Reach.DelayedMethod(owner, sig, pos)`, each by its hash.
pub fn delayed_hash(owner: i32, sig: i32, position: i32) -> i32 {
    case_class_hash("DelayedMethod", &[owner, sig, position])
}

/// `tableSizeFor`: the length of a table for `capacity` elements.
fn table_size_for(capacity: i32) -> usize {
    let at_least = (capacity - 1).max(4) as u32;
    ((1u32 << (31 - at_least.leading_zeros())) as usize * 2).min(1 << 30)
}

fn new_threshold(len: usize) -> usize {
    (len as f64 * LOAD_FACTOR) as usize
}

fn improve(hash: i32) -> i32 {
    hash ^ ((hash as u32) >> 16) as i32
}

/// A table's length and count, and the number of additions so far, which orders elements of
/// one hash.
#[derive(Clone)]
struct Table {
    len: usize,
    threshold: usize,
    size: usize,
    added: u64,
}

impl Table {
    fn new() -> Table {
        let len = table_size_for(16);
        Table { len, threshold: new_threshold(len), size: 0, added: 0 }
    }

    /// The check `add` and `put0` make before an addition, whether or not the element is there.
    fn before_one(&mut self) {
        if self.size + 1 >= self.threshold {
            self.grow(self.len * 2);
        }
    }

    /// `sizeHint(n)`: the table grown to hold `n` elements, whatever it holds already.
    fn size_hint(&mut self, n: usize) {
        let target = table_size_for(((n + 1) as f64 / LOAD_FACTOR) as i32);
        if target > self.len {
            self.grow(target);
        }
    }

    fn grow(&mut self, len: usize) {
        self.threshold = new_threshold(len);
        self.len = len;
    }

    fn next(&mut self) -> u64 {
        self.added += 1;
        self.added
    }

    /// The iteration order of entries given as (improved hash, addition).
    fn order<I: Iterator<Item = (i32, u64)>>(&self, entries: I) -> Vec<usize> {
        let mut keyed: Vec<(usize, i32, u64, usize)> = entries.enumerate().map(|(i, (h, n))| (h as u32 as usize & (self.len - 1), h, n, i)).collect();
        keyed.sort_unstable_by_key(|&(bucket, h, n, _)| (bucket, h, n));
        keyed.into_iter().map(|k| k.3).collect()
    }
}

/// Scala's `mutable.HashSet`, each element with its `hashCode`.
#[derive(Clone)]
pub struct HashSet<T: Copy + Eq + Hash> {
    table: Table,
    entries: Vec<(T, i32, u64)>,
    index: FxMap<T, usize>,
}

impl<T: Copy + Eq + Hash> Default for HashSet<T> {
    fn default() -> Self {
        HashSet { table: Table::new(), entries: Vec::new(), index: FxMap::default() }
    }
}

impl<T: Copy + Eq + Hash> HashSet<T> {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn contains(&self, elem: &T) -> bool {
        self.index.contains_key(elem)
    }

    fn insert(&mut self, elem: T, improved: i32) -> bool {
        if self.index.contains_key(&elem) {
            return false;
        }
        let n = self.table.next();
        self.index.insert(elem, self.entries.len());
        self.entries.push((elem, improved, n));
        self.table.size += 1;
        true
    }

    /// `+=`: true where the element was not there.
    pub fn add(&mut self, elem: T, hash: i32) -> bool {
        self.table.before_one();
        self.insert(elem, improve(hash))
    }

    /// `++=` of another `mutable.HashSet`, in its order.
    pub fn add_all(&mut self, other: &HashSet<T>) {
        self.table.size_hint(other.len());
        for i in other.table.order(other.entries.iter().map(|e| (e.1, e.2))) {
            let (elem, improved, _) = other.entries[i];
            self.insert(elem, improved);
        }
    }

    #[cfg(test)]
    pub fn remove(&mut self, elem: &T) -> bool {
        let Some(i) = self.index.remove(elem) else { return false };
        self.entries.swap_remove(i);
        if let Some(moved) = self.entries.get(i) {
            self.index.insert(moved.0, i);
        }
        self.table.size -= 1;
        true
    }

    /// `clear()`: the elements gone, the table as long as it was.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.index.clear();
        self.table.size = 0;
    }

    #[cfg(test)]
    pub fn table_len(&self) -> usize {
        self.table.len
    }

    /// The elements in the order Scala's iteration gives them.
    pub fn iter(&self) -> Vec<T> {
        self.table.order(self.entries.iter().map(|e| (e.1, e.2))).into_iter().map(|i| self.entries[i].0).collect()
    }
}

/// Scala's `mutable.HashMap`, each key with its `hashCode`.
#[derive(Clone)]
pub struct HashMap<K: Copy + Eq + Hash, V: Clone> {
    table: Table,
    entries: Vec<(K, V, i32, u64)>,
    index: FxMap<K, usize>,
}

impl<K: Copy + Eq + Hash, V: Clone> Default for HashMap<K, V> {
    fn default() -> Self {
        HashMap { table: Table::new(), entries: Vec::new(), index: FxMap::default() }
    }
}

impl<K: Copy + Eq + Hash, V: Clone> HashMap<K, V> {
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.index.get(key).map(|&i| &self.entries[i].1)
    }

    fn put(&mut self, key: K, value: V, improved: i32) {
        match self.index.get(&key) {
            Some(&i) => self.entries[i].1 = value,
            None => {
                let n = self.table.next();
                self.index.insert(key, self.entries.len());
                self.entries.push((key, value, improved, n));
                self.table.size += 1;
            }
        }
    }

    /// `map(key) = value`: a key there keeps its place.
    pub fn update(&mut self, key: K, hash: i32, value: V) {
        self.table.before_one();
        self.put(key, value, improve(hash));
    }

    /// `getOrElseUpdate`.
    pub fn get_or_else_update(&mut self, key: K, hash: i32, value: impl FnOnce() -> V) -> &mut V {
        if !self.index.contains_key(&key) {
            self.table.before_one();
            self.put(key, value(), improve(hash));
        }
        let i = self.index[&key];
        &mut self.entries[i].1
    }

    /// `++=` of another `mutable.HashMap`, in its order.
    pub fn add_all(&mut self, other: &HashMap<K, V>) {
        self.table.size_hint(other.len());
        for i in other.table.order(other.entries.iter().map(|e| (e.2, e.3))) {
            let (key, value, improved, _) = other.entries[i].clone();
            self.table.before_one();
            self.put(key, value, improved);
        }
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        let i = self.index.remove(key)?;
        let (_, value, _, _) = self.entries.swap_remove(i);
        if let Some(moved) = self.entries.get(i) {
            self.index.insert(moved.0, i);
        }
        self.table.size -= 1;
        Some(value)
    }

    /// The entries `keep` refuses removed (`-=` of each).
    pub fn retain(&mut self, mut keep: impl FnMut(&K, &V) -> bool) {
        let gone: Vec<K> = self.entries.iter().filter(|e| !keep(&e.0, &e.1)).map(|e| e.0).collect();
        for key in gone {
            self.remove(&key);
        }
    }

    /// The entries in the order Scala's iteration gives them.
    pub fn iter(&self) -> Vec<(K, V)> {
        self.table.order(self.entries.iter().map(|e| (e.2, e.3))).into_iter().map(|i| (self.entries[i].0, self.entries[i].1.clone())).collect()
    }

    pub fn keys(&self) -> Vec<K> {
        self.table.order(self.entries.iter().map(|e| (e.2, e.3))).into_iter().map(|i| self.entries[i].0).collect()
    }
}

#[cfg(test)]
mod tests {
    //! Held to the real tables and hashes: `tests/native-link/scala-collections.txt` holds what
    //! scala-library 3.8.4 and Scala Native's classes on the JVM gave for generated names, paths,
    //! positions and sequences of operations (its header says how it was made).
    use super::*;

    const VECTORS: &str = include_str!("../../tests/native-link/scala-collections.txt");

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
    }

    fn names() -> Vec<Vec<u8>> {
        let line = VECTORS.lines().find(|l| l.starts_with("# names ")).unwrap();
        line["# names ".len()..].split(' ').map(unhex).collect()
    }

    fn lines(kind: &str) -> impl Iterator<Item = Vec<&str>> + '_ {
        let kind = format!("{} ", kind);
        VECTORS.lines().filter(move |l| l.starts_with(&kind)).map(|l| l.split(' ').collect())
    }

    /// A source and an optional NIR source as the vectors write them: `virtual` or `relative <hex>`,
    /// then `none`, `jar <dir> <path>` or `dir <dir> <path>`.
    fn position_parts(words: &[&str]) -> (Vec<u8>, Option<(i32, i32)>) {
        let bar = words.iter().position(|w| *w == "|").unwrap();
        let file = if words[0] == "virtual" { Vec::new() } else { unhex(words[1]) };
        let nir = match words[bar + 1] {
            "none" => None,
            "jar" => Some((unix_path_hash(&unhex(words[bar + 2])), zip_path_hash(&unhex(words[bar + 3])))),
            _ => Some((unix_path_hash(&unhex(words[bar + 2])), unix_path_hash(&unhex(words[bar + 3])))),
        };
        (file, nir)
    }

    #[test]
    fn hashes_are_the_jvms() {
        let mut checked = 0;
        for w in lines("sig") {
            assert_eq!(string_hash(&unhex(w[2])), w[1].parse::<i32>().unwrap(), "sig {}", w[2]);
            checked += 1;
        }
        for w in lines("top") {
            assert_eq!(top_hash(&unhex(w[2])), w[1].parse::<i32>().unwrap());
            checked += 1;
        }
        for w in lines("member") {
            assert_eq!(member_hash(top_hash(&unhex(w[2])), string_hash(&unhex(w[3]))), w[1].parse::<i32>().unwrap());
            checked += 1;
        }
        for w in lines("unixpath") {
            assert_eq!(unix_path_hash(&unhex(w[2])), w[1].parse::<i32>().unwrap());
            checked += 1;
        }
        for w in lines("zippath") {
            assert_eq!(zip_path_hash(&unhex(w[2])), w[1].parse::<i32>().unwrap());
            checked += 1;
        }
        for w in lines("position") {
            let (file, nir) = position_parts(&w[4..]);
            assert_eq!(position_hash(&file, w[2].parse().unwrap(), w[3].parse().unwrap(), nir), w[1].parse::<i32>().unwrap(), "{:?}", w);
            checked += 1;
        }
        for w in lines("delayed") {
            let (file, nir) = position_parts(&w[6..]);
            let position = position_hash(&file, w[4].parse().unwrap(), w[5].parse().unwrap(), nir);
            assert_eq!(delayed_hash(top_hash(&unhex(w[2])), string_hash(&unhex(w[3])), position), w[1].parse::<i32>().unwrap(), "{:?}", w);
            checked += 1;
        }
        for w in lines("type") {
            let shown = String::from_utf8(unhex(w[2])).unwrap();
            let hash = match shown.strip_prefix("@\"T") {
                Some(rest) => {
                    let name = rest.trim_end_matches('"').trim_start_matches(|c: char| c.is_ascii_digit());
                    ref_type_hash(name.as_bytes())
                }
                None => {
                    let mut name = shown.clone();
                    name[..1].make_ascii_uppercase();
                    case_object_hash(&name)
                }
            };
            assert_eq!(hash, w[1].parse::<i32>().unwrap(), "type {}", shown);
            checked += 1;
        }
        assert!(checked > 340, "{} hashes checked", checked);
    }

    #[test]
    fn immutable_maps_iterate_as_scalas() {
        let names = names();
        let mut cases = 0;
        for line in VECTORS.lines().filter(|l| l.starts_with("imap ")) {
            let (keys, expected) = line["imap ".len()..].split_once(" = ").unwrap();
            let keys: Vec<usize> = keys.split(',').map(|k| k.parse().unwrap()).collect();
            let hashes: Vec<i32> = keys.iter().map(|&k| string_hash(&names[k])).collect();
            let got: Vec<String> = immutable_order(&hashes).into_iter().map(|i| keys[i].to_string()).collect();
            assert_eq!(got.join(","), expected, "{}", line);
            cases += 1;
        }
        assert!(cases >= 60);
    }

    /// The operations of a vector's case, `+3`, `-3`, `=3`, `?3`, `++` and `clear`, on names by index.
    fn ops(text: &str) -> Vec<(&str, usize)> {
        text.split(',').filter(|o| !o.is_empty()).map(|o| match o {
            "++" | "clear" => (o, 0),
            _ => (&o[..1], o[1..].parse().unwrap()),
        }).collect()
    }

    #[test]
    fn sets_iterate_as_scalas() {
        let names = names();
        let hash = |i: usize| string_hash(&names[i]);
        let mut cases = 0;
        for line in VECTORS.lines().filter(|l| l.starts_with("set ")) {
            let (head, expected) = line["set ".len()..].split_once(" = ").unwrap();
            let (source_ops, own_ops) = head.split_once(" ; ").unwrap();
            let mut source = HashSet::default();
            for (_, i) in ops(source_ops) {
                source.add(i, hash(i));
            }
            let mut set = HashSet::default();
            for (op, i) in ops(own_ops) {
                match op {
                    "++" => set.add_all(&source),
                    "clear" => set.clear(),
                    "+" => {
                        set.add(i, hash(i));
                    }
                    _ => {
                        set.remove(&i);
                    }
                }
            }
            let got: Vec<String> = set.iter().iter().map(|i| i.to_string()).collect();
            assert_eq!(got.join(","), expected, "{}", line);
            cases += 1;
        }
        assert!(cases >= 100);
    }

    #[test]
    fn maps_iterate_as_scalas() {
        let names = names();
        let hash = |i: usize| string_hash(&names[i]);
        let mut cases = 0;
        for line in VECTORS.lines().filter(|l| l.starts_with("map ")) {
            let (head, expected) = line["map ".len()..].split_once(" = ").unwrap();
            let (source_ops, own_ops) = head.split_once(" ; ").unwrap();
            let mut source = HashMap::default();
            for (n, (_, i)) in ops(source_ops).into_iter().enumerate() {
                source.update(i, hash(i), n);
            }
            let mut map = HashMap::default();
            for (n, (op, i)) in ops(own_ops).into_iter().enumerate() {
                match op {
                    "++" => map.add_all(&source),
                    "=" => map.update(i, hash(i), 1000 + n),
                    "?" => {
                        map.get_or_else_update(i, hash(i), || 1000 + n);
                    }
                    _ => {
                        map.remove(&i);
                    }
                }
            }
            let got: Vec<String> = map.iter().iter().map(|(k, v)| format!("{}:{}", k, v)).collect();
            assert_eq!(got.join(","), expected, "{}", line);
            cases += 1;
        }
        assert!(cases >= 100);
    }
}
