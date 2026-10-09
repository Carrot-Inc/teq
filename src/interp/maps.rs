//! The store behind `RawMap`: an insertion-ordered map whose keys compare as `==` does, so
//! that `1`, `1L` and `1.0` find each other's entries. A removed entry leaves a hole, which
//! keeps the order of the rest.

use super::value::*;
use super::*;

impl<'a, 't> Interp<'a, 't> {
    pub(super) fn map_find(&mut self, m: &Rc<MapCell>, key: &Value) -> R<Option<usize>> {
        let h = self.hash(key)?;
        // Two plain keys are `==` exactly when they are the same value, so those compare inside
        // the borrow; a key that may run `equals` (an object, a char against a number) is
        // compared after it.
        let mut candidates: Vec<(usize, Value)> = Vec::new();
        {
            let map = m.borrow();
            let Some(bucket) = map.index.get(&h) else { return Ok(None) };
            let plain = plain_key(key);
            for &i in bucket {
                let Some((k, _)) = map.entries[i as usize].as_ref() else { continue };
                if plain && plain_key(k) {
                    if k.same(key) {
                        return Ok(Some(i as usize));
                    }
                } else {
                    candidates.push((i as usize, k.clone()));
                }
            }
        }
        for (i, k) in candidates {
            if self.equal(&k, key)? {
                return Ok(Some(i));
            }
        }
        Ok(None)
    }

    pub(super) fn map_has(&mut self, m: &Rc<MapCell>, key: &Value) -> R<bool> {
        Ok(self.map_find(m, key)?.is_some())
    }

    pub(super) fn map_get(&mut self, m: &Rc<MapCell>, key: &Value) -> R {
        match self.map_find(m, key)? {
            Some(i) => Ok(m.borrow().entries[i].as_ref().map(|(_, v)| v.clone()).unwrap_or(Value::Unit)),
            None => Ok(Value::Unit),
        }
    }

    pub(super) fn map_set(&mut self, m: &Rc<MapCell>, key: Value, value: Value) -> R<()> {
        if let Some(i) = self.map_find(m, &key)? {
            if let Some(entry) = m.write().entries[i].as_mut() {
                entry.1 = value;
            }
            return Ok(());
        }
        let h = self.hash(&key)?;
        let mut map = m.write();
        if map.entries.len() > 2 * map.live + 16 {
            Self::compact(&mut map, &mut |k| prim_hash(k));
        }
        let i = map.entries.len() as u32;
        map.entries.push(Some((key, value)));
        map.index.entry(h).or_default().push(i);
        map.live += 1;
        Ok(())
    }

    /// Drops the holes; keys of objects keep their hash through `hashes`, computed before the
    /// borrow since `hashCode` may run program code.
    fn compact(map: &mut HMap, _hashes: &mut dyn FnMut(&Value) -> Option<i32>) {
        let mut positions: Vec<(u32, u32)> = Vec::new();
        let mut next = 0u32;
        let mut entries = Vec::with_capacity(map.live);
        for (old, e) in map.entries.drain(..).enumerate() {
            if let Some(e) = e {
                positions.push((old as u32, next));
                entries.push(Some(e));
                next += 1;
            }
        }
        map.entries = entries;
        let remap: FxMap<u32, u32> = positions.into_iter().collect();
        for bucket in map.index.values_mut() {
            bucket.retain(|i| remap.contains_key(i));
            for i in bucket.iter_mut() {
                *i = remap[i];
            }
        }
        map.index.retain(|_, b| !b.is_empty());
    }

    pub(super) fn map_delete(&mut self, m: &Rc<MapCell>, key: &Value) -> R<bool> {
        let Some(i) = self.map_find(m, key)? else { return Ok(false) };
        let h = self.hash(key)?;
        let mut map = m.write();
        map.entries[i] = None;
        map.live -= 1;
        if let Some(bucket) = map.index.get_mut(&h) {
            bucket.retain(|&j| j as usize != i);
            if bucket.is_empty() {
                map.index.remove(&h);
            }
        }
        Ok(true)
    }

    pub(super) fn map_keys(m: &Rc<MapCell>) -> Vec<Value> {
        m.borrow().entries.iter().flatten().map(|(k, _)| k.clone()).collect()
    }

    pub(super) fn map_values(m: &Rc<MapCell>) -> Vec<Value> {
        m.borrow().entries.iter().flatten().map(|(_, v)| v.clone()).collect()
    }

    pub(super) fn map_entries(m: &Rc<MapCell>) -> Vec<(Value, Value)> {
        m.borrow().entries.iter().flatten().cloned().collect()
    }

    pub(super) fn map_size(m: &Rc<MapCell>) -> usize {
        m.borrow().live
    }

    pub(super) fn map_clear(m: &Rc<MapCell>) {
        let mut map = m.write();
        map.entries.clear();
        map.index.clear();
        map.live = 0;
    }

    pub(super) fn map_equals(&mut self, a: &Rc<MapCell>, b: &Rc<MapCell>) -> R<bool> {
        if Self::map_size(a) != Self::map_size(b) {
            return Ok(false);
        }
        for (k, v) in Self::map_entries(a) {
            if !self.map_has(b, &k)? {
                return Ok(false);
            }
            let w = self.map_get(b, &k)?;
            if !self.equal(&v, &w)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// MurmurHash3's `mapHash`: the unordered hash of the entries as `Tuple2`s.
    pub(super) fn map_hash(&mut self, m: &Rc<MapCell>) -> R<i32> {
        let (mut a, mut b, mut c, mut n) = (0i32, 0i32, 1i32, 0i32);
        for (k, v) in Self::map_entries(m) {
            let (hk, hv) = (self.hash(&k)?, self.hash(&v)?);
            let e = finalize_hash(mix(mix(mix(PRODUCT_SEED, str_hash("Tuple2")), hk), hv), 2);
            a = a.wrapping_add(e);
            b ^= e;
            c = c.wrapping_mul(e | 1);
            n += 1;
        }
        Ok(unordered_hash(a, b, c, n, str_hash("Map")))
    }
}

/// A key whose `==` is its value: a string, a boolean, a number or the two units.
fn plain_key(v: &Value) -> bool {
    matches!(v, Value::Str(_) | Value::Bool(_) | Value::Int(_) | Value::Long(_) | Value::Double(_) | Value::Float(_) | Value::Byte(_) | Value::Short(_) | Value::Null | Value::Unit)
}
