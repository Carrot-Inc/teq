//! Native runs of the standard library's hot collection methods, in place of their Scala
//! bodies: `List.map` and its family, `Option`, the folds over arrays and strings,
//! `Range.foreach`, the `IterableOps` operations through a native traversal, `Set.from` and
//! `Map.from` over the raw store their bodies build. Each follows the body it stands
//! for (the calls it makes, in their order, and what it builds) and answers `None` before any
//! effect when the receiver is not what it expects, so that the body runs then. A def is
//! matched by the qualified name a builtin would have (`Interp::qualified_name`), once per
//! function.

use super::value::*;
use super::*;
use std::cell::Cell;

pub(super) type Native = fn(&mut Interp<'_, '_>, &Value, &[Value]) -> R<Option<Value>>;

const UNKNOWN: u32 = u32::MAX;
const NONE: u32 = u32::MAX - 1;

pub(super) struct ListShape {
    pub cons: ClassId,
    pub nil: Rc<Object>,
    pub slots: Rc<[u32]>,
}

pub(super) struct OptionShape {
    pub some: ClassId,
    pub none: Rc<Object>,
    pub slots: Rc<[u32]>,
}

/// The classes of the immutable stores and what they hold, with the slots of their
/// constructor parameters, which are their fields: `PersistentMap` (index, entries, dead,
/// fallback, compacted), `HashSet` (index, entries, dead, compacted), `Vector` (root, depth,
/// offset, trieLength, items, itemCount, chunked), `TrieNode` (dataMap, nodeMap, content) and
/// `Tuple2`.
pub(super) struct StoreShapes {
    pub map: (ClassId, Rc<[u32]>),
    pub set: (ClassId, Rc<[u32]>),
    pub vector: (ClassId, Rc<[u32]>),
    pub trie: (ClassId, Rc<[u32]>),
    pub tuple2: (ClassId, Rc<[u32]>),
}

/// `TEQ_NO_NATIVES=1` runs every std body as its Scala says, for the check that the natives
/// change no output (`tests/split.sh`); any other value names the natives to leave out, as a
/// comma-separated list of parts of their qualified names.
fn native_off(name: &str) -> bool {
    static OFF: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    let off = OFF.get_or_init(|| std::env::var("TEQ_NO_NATIVES").map(|v| v.split(',').filter(|p| !p.is_empty()).map(str::to_string).collect()).unwrap_or_default());
    off.iter().any(|part| part == "1" || part == "all" || name.contains(part.as_str()))
}

pub(super) const NATIVES: &[(&str, Native)] = &[
    ("scala.List.::", list_cons),
    ("scala.::.isEmpty", |_, _, _| Ok(Some(Value::Bool(false)))),
    ("scala.Nil.isEmpty", |_, _, _| Ok(Some(Value::Bool(true)))),
    ("scala.List.map", list_map),
    ("scala.List.filter", |it, this, a| list_filter(it, this, a, false)),
    ("scala.List.filterNot", |it, this, a| list_filter(it, this, a, true)),
    ("scala.List.withFilter", |it, this, a| list_filter(it, this, a, false)),
    ("scala.List.flatMap", list_flat_map),
    ("scala.List.foreach", list_foreach),
    ("scala.List.foldLeft", list_fold_left),
    ("scala.List.exists", |it, this, a| list_exists(it, this, a, false)),
    ("scala.List.forall", |it, this, a| list_exists(it, this, a, true)),
    ("scala.List.find", list_find),
    ("scala.List.collectFirst", list_collect_first),
    ("scala.List.contains", list_contains),
    ("scala.List.length", list_length),
    ("scala.List.reverse", list_reverse),
    ("scala.List.take", list_take),
    ("scala.List.drop", list_drop),
    ("scala.List.:::", list_prepend_all),
    ("scala.List.headOption", list_head_option),
    ("scala.List.zipWithIndex", list_zip_with_index),
    ("scala.fromArray", from_array),
    ("scala.iterableToArray", iterable_to_array),
    ("scala.joinStrings", join_strings),
    ("scala.Array.foreach", array_foreach),
    ("scala.Array.foldLeft", array_fold_left),
    ("scala.Array.find", array_find),
    ("scala.Array.exists", array_exists),
    ("scala.Array.toList", array_to_list),
    ("scala.String.foldLeft", string_fold_left),
    ("scala.Option.map", option_map),
    ("scala.Option.flatMap", option_flat_map),
    ("scala.Option.getOrElse", option_get_or_else),
    ("scala.Option.orElse", option_or_else),
    ("scala.Option.foreach", option_foreach),
    ("scala.Option.isEmpty", |it, this, _| option_test(it, this, true)),
    ("scala.Option.isDefined", |it, this, _| option_test(it, this, false)),
    ("scala.Option.nonEmpty", |it, this, _| option_test(it, this, false)),
    ("scala.Option.exists", option_exists),
    ("scala.Option.forall", option_forall),
    ("scala.Option.filter", option_filter),
    ("scala.Option.fold", option_fold),
    ("scala.Option.toList", option_to_list),
    ("scala.Range.foreach", range_foreach),
    ("scala.Range.length", range_length),
    ("scala.Range.isEmpty", range_is_empty),
    ("scala.Vector.foreach", |it, this, a| items_foreach(it, this, a, "Vector")),
    ("scala.Vector.map", vector_map),
    ("scala.Vector.toList", |it, this, _| items_to_list(it, this, "Vector")),
    ("scala.ArraySeq.foreach", |it, this, a| items_foreach(it, this, a, "ArraySeq")),
    ("scala.ArraySeq.toList", |it, this, _| items_to_list(it, this, "ArraySeq")),
    ("scala.collection.mutable.ArrayBuffer.foreach", |it, this, a| items_foreach(it, this, a, "ArrayBuffer")),
    ("scala.PersistentMap.ordinalOf", |it, this, a| store_ordinal(it, this, a, true)),
    ("scala.PersistentMap.get", map_get),
    ("scala.PersistentMap.contains", |it, this, a| store_contains(it, this, a, true)),
    ("scala.PersistentMap.isDefinedAt", |it, this, a| store_contains(it, this, a, true)),
    ("scala.PersistentMap.getOrElse", map_get_or_else),
    ("scala.PersistentMap.foreach", |it, this, a| store_foreach(it, this, a, true)),
    ("scala.PersistentMap.toArray", store_to_array),
    ("scala.PersistentMap.keys", |it, this, _| map_parts(it, this, 0)),
    ("scala.PersistentMap.values", |it, this, _| map_parts(it, this, 1)),
    ("scala.collection.immutable.HashSet.ordinalOf", |it, this, a| store_ordinal(it, this, a, false)),
    ("scala.collection.immutable.HashSet.contains", |it, this, a| store_contains(it, this, a, false)),
    ("scala.collection.immutable.HashSet.foreach", |it, this, a| store_foreach(it, this, a, false)),
    ("scala.collection.immutable.HashSet.toArray", store_to_array),
    ("scala.Vector.apply", vector_apply),
    ("scala.trieFind", trie_find_native),
    ("scala.trieInsert", trie_insert_native),
    ("scala.trieRemove", trie_remove_native),
    ("scala.trieOf", trie_of_native),
    ("scala.trieRebuilt", trie_rebuilt_native),
    ("scala.PersistentMap.fromEntries", |it, _, a| store_of(it, a, true)),
    ("scala.collection.immutable.HashSet.fromDistinct", |it, _, a| store_of(it, a, false)),
    ("scala.Set.from", set_from),
    ("scala.Map.from", map_from),
    ("scala.Map.fromRaw", map_from_raw),
    ("scala.IterableOps.toList", iterable_to_list),
    ("scala.IterableOps.toArray", |it, this, _| iterable_to_array(it, &Value::Unit, std::slice::from_ref(this))),
    ("scala.IterableOps.toSet", |it, this, _| set_of_elements(it, this)),
    ("scala.IterableOnce.toMap", |it, this, _| map_of_elements(it, this)),
    ("scala.IterableOps.map", iterable_map),
    ("scala.IterableOps.flatMap", iterable_flat_map),
    ("scala.IterableOps.flatten", iterable_flat_map),
    ("scala.IterableOps.filter", |it, this, a| iterable_filter(it, this, a, false)),
    ("scala.IterableOps.filterNot", |it, this, a| iterable_filter(it, this, a, true)),
    ("scala.IterableOps.withFilter", |it, this, a| iterable_filter(it, this, a, false)),
    ("scala.IterableOps.foldLeft", iterable_fold_left),
    ("scala.IterableOps.exists", |it, this, a| iterable_exists(it, this, a, false)),
    ("scala.IterableOps.forall", |it, this, a| iterable_exists(it, this, a, true)),
    ("scala.IterableOps.find", iterable_find),
    ("scala.IterableOps.collectFirst", iterable_collect_first),
    ("scala.IterableOps.size", iterable_size),
    ("scala.IterableOps.isEmpty", |it, this, _| iterable_empty(it, this, true)),
    ("scala.IterableOps.nonEmpty", |it, this, _| iterable_empty(it, this, false)),
    ("scala.IterableOps.headOption", iterable_head_option),
    ("scala.IterableOps.mkString", iterable_mk_string),
    ("scala.IterableOps.groupBy", iterable_group_by),
    ("scala.IterableOps.groupMap", iterable_group_map),
    ("scala.IterableOps.distinctBy", iterable_distinct_by),
    ("scala.IterableOps.distinct", |it, this, _| distinct_by(it, this, None)),
    ("scala.SeqOps.distinctBy", iterable_distinct_by),
    ("scala.SeqOps.distinct", |it, this, _| distinct_by(it, this, None)),
    ("scala.Seq.hashCode", seq_hash_code),
    ("scala.Vector.wrap", vector_wrap),
    ("scala.Vector.buildCC", vector_wrap),
    ("scala.Range.buildCC", vector_wrap),
    ("scala.IndexedSeq.buildCC", vector_wrap),
    ("scala.IterableOps.buildC", iterable_build_c),
    ("scala.List.buildCC", |it, _, a| from_array(it, &Value::Unit, a)),
    ("java.nio.file.Path.of", path_of),
    ("java.nio.file.Path.getParent", path_parent),
    ("java.nio.file.Path.getFileName", path_file_name),
    ("java.nio.file.Path.isAbsolute", path_is_absolute),
    ("java.nio.file.Path.resolve", path_resolve),
    ("java.util.Collection.toArray", array_list_to_array),
    ("scala.quoted.ToExpr.lit", to_expr_lit),
    ("scala.quoted.FromExpr.constant", from_expr_constant),
    ("scala.quoted.Reflect.TreeMethods.Tree.asExpr", tree_as_expr),
    ("scala.quoted.Reflect.TreeMethods.Tree.asExprOf", tree_as_expr_of),
    ("scala.quoted.Quotes.Expr.asExprOf", expr_as_expr_of),
];

fn arg(a: &[Value], i: usize) -> Value {
    a.get(i).cloned().unwrap_or(Value::Null)
}

impl<'a, 't> Interp<'a, 't> {
    /// The native run of function `f`, looked up by its qualified name once.
    #[inline]
    pub(super) fn native_of(&mut self, f: FunId) -> Option<Native> {
        let i = f.idx();
        match self.fun_natives[i] {
            NONE => None,
            UNKNOWN => {
                let sym = self.prog().funs[i].sym;
                let name = self.qualified_name(sym);
                let found = NATIVES.iter().position(|(n, _)| *n == &*name && !native_off(n));
                self.fun_natives[i] = found.map_or(NONE, |k| k as u32);
                found.map(|k| NATIVES[k].1)
            }
            k => Some(NATIVES[k as usize].1),
        }
    }

    // ---- shapes ----

    pub(super) fn list_shape(&mut self) -> Option<Rc<ListShape>> {
        if let Some(s) = &self.list_shape {
            return s.clone();
        }
        let shape = (|| {
            let cons = self.known_class("::")?;
            let nil = self.known_class("Nil")?;
            let slots = self.ctor_plan(cons)?;
            if slots.len() != 2 {
                return None;
            }
            let Ok(Value::Obj(nil)) = self.module(nil) else { return None };
            Some(Rc::new(ListShape { cons, nil, slots }))
        })();
        self.list_shape = Some(shape.clone());
        shape
    }

    pub(super) fn option_shape(&mut self) -> Option<Rc<OptionShape>> {
        if let Some(s) = &self.option_shape {
            return s.clone();
        }
        let shape = (|| {
            let some = self.known_class("Some")?;
            let none = self.known_class("None")?;
            let slots = self.ctor_plan(some)?;
            if slots.len() != 1 {
                return None;
            }
            let Ok(Value::Obj(none)) = self.module(none) else { return None };
            Some(Rc::new(OptionShape { some, none, slots }))
        })();
        self.option_shape = Some(shape.clone());
        shape
    }

    pub(super) fn store_shapes(&mut self) -> Option<Rc<StoreShapes>> {
        if let Some(s) = &self.store_shapes {
            return s.clone();
        }
        let shapes = (|| {
            let map = self.shape_of("PersistentMap", &["index", "entries", "dead", "fallback", "compacted"])?;
            let set = self.shape_of("HashSet", &["index", "entries", "dead", "compacted"])?;
            let vector = self.shape_of("Vector", &["root", "depth", "offset", "trieLength", "items", "itemCount", "chunked"])?;
            let trie = self.shape_of("TrieNode", &["dataMap", "nodeMap", "content"])?;
            let tuple2 = self.shape_of("Tuple2", &["_1", "_2"])?;
            Some(Rc::new(StoreShapes { map, set, vector, trie, tuple2 }))
        })();
        self.store_shapes = Some(shapes.clone());
        shapes
    }

    /// The slots of the constructor parameters of the class named `name` in the std, when it
    /// has a plain constructor.
    fn plain_class(&mut self, name: &'static str) -> Option<(ClassId, Rc<[u32]>)> {
        if let Some(found) = self.plain_classes.get(name) {
            return found.clone();
        }
        let found = self.known_class(name).and_then(|c| self.ctor_plan(c).map(|slots| (c, slots)));
        self.plain_classes.insert(name, found.clone());
        found
    }

    /// The slot of the field `name` of class `c`, once an instance has it.
    fn field_slot_named(&mut self, c: ClassId, name: &'static str) -> Option<u32> {
        if let Some(&slot) = self.named_slots.get(&(c, name)) {
            return Some(slot);
        }
        let n = self.typer.interner.intern(name);
        let sym = *self.syms().class(c).members.get(&n)?;
        let key = self.field_key(sym);
        let slot = self.slot_if_known(c, key)?;
        self.named_slots.insert((c, name), slot);
        Some(slot)
    }

    fn field_of(&mut self, o: &Rc<Object>, name: &'static str) -> Option<Value> {
        let slot = self.field_slot_named(o.class, name)?;
        let v = o.fields.borrow().get(slot as usize).cloned()?;
        if matches!(v, Value::Absent) { None } else { Some(v) }
    }

    /// The value in `slot` of `o`, unless no write reached it.
    fn slot_value(o: &Rc<Object>, slot: u32) -> Option<Value> {
        let v = o.fields.borrow().get(slot as usize).cloned()?;
        if matches!(v, Value::Absent) { None } else { Some(v) }
    }

    /// The plan of a std class whose constructor parameters are its fields, when they are the
    /// `params` named, in that order: a native reads a field by the slot of its parameter, so
    /// a renamed or reordered parameter must fail here rather than read the wrong field.
    fn shape_of(&mut self, name: &'static str, params: &[&str]) -> Option<(ClassId, Rc<[u32]>)> {
        let plan = self.plain_class(name)?;
        if plan.1.len() != params.len() {
            return None;
        }
        let ti = self.tclass_index(plan.0)?;
        let ctor_params = self.prog().classes[ti].ctor_params.clone();
        if ctor_params.len() != params.len() {
            return None;
        }
        for (p, expected) in ctor_params.iter().zip(params) {
            let n = self.typer.interner.intern(expected);
            if self.syms().sym(*p).name != n {
                return None;
            }
        }
        Some(plan)
    }

    /// The item at `i` of a `Vector` (root, depth, offset, trieLength, items, itemCount,
    /// chunked): from its tail, or down one path of its trie, as `Vector.apply` walks it.
    fn vector_item(&mut self, o: &Rc<Object>, i: i32) -> Option<Value> {
        let shapes = self.store_shapes()?;
        let (vector, slots) = (shapes.vector.0, &shapes.vector.1);
        if o.class != vector || i < 0 {
            return None;
        }
        let (Value::Int(depth), Value::Int(offset), Value::Int(trie_length), Value::Int(count)) =
            (Self::slot_value(o, slots[1])?, Self::slot_value(o, slots[2])?, Self::slot_value(o, slots[3])?, Self::slot_value(o, slots[5])?)
        else {
            return None;
        };
        if i >= trie_length {
            if i - trie_length >= count {
                return None;
            }
            let Value::Array(items) = Self::slot_value(o, slots[4])? else { return None };
            let items = items.borrow();
            return items.get((i - trie_length) as usize).cloned();
        }
        let p = (i + offset) as u32;
        let mut node = Self::slot_value(o, slots[0])?;
        let mut level = depth;
        while level > 0 {
            let Value::Array(children) = node else { return None };
            let next = children.borrow().get(((p >> (5 * level as u32)) & 31) as usize).cloned()?;
            node = next;
            level -= 1;
        }
        let Value::Array(leaf) = node else { return None };
        let leaf = leaf.borrow();
        leaf.get((p & 31) as usize).cloned()
    }

    /// The length of a `Vector`.
    fn vector_length(&mut self, o: &Rc<Object>) -> Option<i32> {
        let shapes = self.store_shapes()?;
        let slots = &shapes.vector.1;
        if o.class != shapes.vector.0 {
            return None;
        }
        let (Value::Int(trie_length), Value::Int(count)) = (Self::slot_value(o, slots[3])?, Self::slot_value(o, slots[5])?) else { return None };
        Some(trie_length + count)
    }

    /// The size of a collection whose shape carries it: a `List`, an `Option`, the array-backed
    /// sequences, a `Vector`, a `Range`, an immutable `Map` or `Set`.
    fn known_size(&mut self, v: &Value) -> Option<usize> {
        let Value::Obj(o) = v else { return None };
        if let Some(shape) = self.list_shape() {
            if Rc::ptr_eq(o, &shape.nil) {
                return Some(0);
            }
            if o.class == shape.cons {
                return self.list_vec(&shape, v).map(|items| items.len());
            }
        }
        if let Some(shape) = self.option_shape() {
            if Rc::ptr_eq(o, &shape.none) {
                return Some(0);
            }
            if o.class == shape.some {
                return Some(1);
            }
        }
        if let Some(n) = self.vector_length(o) {
            return Some(n.max(0) as usize);
        }
        for class in ["ArraySeq", "ArrayBuffer"] {
            if self.known_class(class) == Some(o.class) {
                let Value::Array(items) = self.field_of(o, "items")? else { return None };
                let n = items.borrow().len();
                return Some(n);
            }
        }
        if let Some(shapes) = self.store_shapes() {
            let slots = if o.class == shapes.map.0 { Some(&shapes.map.1) } else if o.class == shapes.set.0 { Some(&shapes.set.1) } else { None };
            if let Some(slots) = slots {
                let (Value::Obj(entries), Value::Int(dead)) = (Self::slot_value(o, slots[1])?, Self::slot_value(o, slots[2])?) else { return None };
                let n = self.vector_length(&entries)?;
                return Some((n - dead).max(0) as usize);
            }
        }
        if let Some((start, limit, step)) = self.range_of(o) {
            if step == 0 {
                return None;
            }
            let n = if step > 0 { if start >= limit { 0 } else { (limit as i64 - start as i64 + step as i64 - 1) / step as i64 } } else if start <= limit { 0 } else { (start as i64 - limit as i64 - step as i64 - 1) / (-(step as i64)) };
            return Some(n as usize);
        }
        None
    }

    /// The items of a `Vector`, `ArraySeq` or `ArrayBuffer`: its array, cut to its count
    /// where the class keeps one; a `Vector` with a trie root answers `None`.
    fn items_of(&mut self, o: &Rc<Object>, class: &'static str) -> Option<Vec<Value>> {
        if class == "Vector" {
            let shapes = self.store_shapes()?;
            let slots = &shapes.vector.1;
            if o.class != shapes.vector.0 || !matches!(Self::slot_value(o, slots[0])?, Value::Null) {
                return None;
            }
            let Value::Int(n) = Self::slot_value(o, slots[5])? else { return None };
            let Value::Array(items) = Self::slot_value(o, slots[4])? else { return None };
            let items = items.borrow();
            let n = (n.max(0) as usize).min(items.len());
            return Some(items[..n].to_vec());
        }
        if !self.known_class(class).map_or(false, |c| c == o.class) {
            return None;
        }
        let Value::Array(items) = self.field_of(o, "items")? else { return None };
        let items = items.borrow();
        Some(items.clone())
    }

    fn range_of(&mut self, o: &Rc<Object>) -> Option<(i32, i32, i32)> {
        if !self.known_class("Range").map_or(false, |c| c == o.class) {
            return None;
        }
        let (Value::Int(start), Value::Int(end), Value::Int(step), Value::Bool(inclusive)) =
            (self.field_of(o, "start")?, self.field_of(o, "end")?, self.field_of(o, "step")?, self.field_of(o, "isInclusive")?)
        else {
            return None;
        };
        let limit = if !inclusive { end } else if step > 0 { end.wrapping_add(1) } else { end.wrapping_sub(1) };
        Some((start, limit, step))
    }

    // ---- lists ----

    pub(super) fn list_vec(&self, shape: &ListShape, v: &Value) -> Option<Vec<Value>> {
        let mut out = Vec::new();
        let mut cur = v.clone();
        loop {
            let Value::Obj(o) = &cur else { return None };
            if Rc::ptr_eq(o, &shape.nil) {
                return Some(out);
            }
            if o.class != shape.cons {
                return None;
            }
            let next = {
                let fields = o.fields.borrow();
                match (fields.get(shape.slots[0] as usize), fields.get(shape.slots[1] as usize)) {
                    (Some(h), Some(t)) if !matches!(h, Value::Absent) && !matches!(t, Value::Absent) => {
                        out.push(h.clone());
                        t.clone()
                    }
                    _ => return None,
                }
            };
            cur = next;
        }
    }

    #[inline]
    fn cons(&self, shape: &ListShape, head: Value, tail: Value) -> Value {
        let n = shape.slots.iter().copied().max().unwrap_or(0) as usize + 1;
        let mut fields = Vec::with_capacity(n);
        fields.resize(n, Value::Absent);
        fields[shape.slots[0] as usize] = head;
        fields[shape.slots[1] as usize] = tail;
        Value::Obj(Rc::new(Object { class: shape.cons, fields: RefCell::new(fields), env: None, name: None, ordinal: Cell::new(0), hash: Cell::new(0), slot: Cell::new(0), born: Cell::new(0), made: Cell::new(super::made_now()) }))
    }

    pub(super) fn vec_list(&self, shape: &ListShape, items: Vec<Value>) -> Value {
        self.vec_list_onto(shape, items, Value::Obj(shape.nil.clone()))
    }

    fn vec_list_onto(&self, shape: &ListShape, items: Vec<Value>, tail: Value) -> Value {
        let mut acc = tail;
        for x in items.into_iter().rev() {
            acc = self.cons(shape, x, acc);
        }
        acc
    }

    fn option_of(&mut self, shape: &OptionShape, v: Option<Value>) -> Value {
        match v {
            Some(v) => self.construct_plain(shape.some, &shape.slots, vec![v]),
            None => Value::Obj(shape.none.clone()),
        }
    }

    /// `Some(v)` or `None` read from an option value; `None` (outer) for anything else.
    fn option_value(&self, shape: &OptionShape, v: &Value) -> Option<Option<Value>> {
        let Value::Obj(o) = v else { return None };
        if Rc::ptr_eq(o, &shape.none) {
            return Some(None);
        }
        if o.class != shape.some {
            return None;
        }
        let x = o.fields.borrow().get(shape.slots[0] as usize).cloned()?;
        if matches!(x, Value::Absent) { None } else { Some(Some(x)) }
    }

    fn tuple2(&mut self, a: Value, b: Value) -> Option<Value> {
        let shapes = self.store_shapes()?;
        Some(self.construct_plain(shapes.tuple2.0, &shapes.tuple2.1, vec![a, b]))
    }

    fn tuple2_parts(&mut self, v: &Value) -> Option<(Value, Value)> {
        let shapes = self.store_shapes()?;
        let (c, slots) = (shapes.tuple2.0, &shapes.tuple2.1);
        let Value::Obj(o) = v else { return None };
        if o.class != c {
            return None;
        }
        let fields = o.fields.borrow();
        match (fields.get(slots[0] as usize), fields.get(slots[1] as usize)) {
            (Some(a), Some(b)) if !matches!(a, Value::Absent) && !matches!(b, Value::Absent) => Some((a.clone(), b.clone())),
            _ => None,
        }
    }

    // ---- traversal ----

    /// The elements of a collection of the std by its shape, without running its `iterator`:
    /// a list, an option, an array-backed sequence, a set or map over a raw store, a range.
    /// Anything else, a view or a lazy list among them, is `None`: its traversal may be lazy
    /// or have effects of its own.
    pub(super) fn elements_fast(&mut self, v: &Value) -> R<Option<Vec<Value>>> {
        let Value::Obj(o) = v else { return Ok(None) };
        if let Some(shape) = self.list_shape() {
            if let Some(items) = self.list_vec(&shape, v) {
                return Ok(Some(items));
            }
        }
        if let Some(shape) = self.option_shape() {
            if let Some(x) = self.option_value(&shape, v) {
                return Ok(Some(x.into_iter().collect()));
            }
        }
        for class in ["Vector", "ArraySeq", "ArrayBuffer"] {
            if let Some(items) = self.items_of(o, class) {
                return Ok(Some(items));
            }
        }
        if let Some(shapes) = self.store_shapes() {
            if o.class == shapes.map.0 || o.class == shapes.set.0 {
                return Ok(store_entries(self, v));
            }
        }
        if let Some((start, limit, step)) = self.range_of(o) {
            let mut out = Vec::new();
            let mut i = start;
            if step > 0 {
                while i < limit {
                    out.push(Value::Int(i));
                    i = i.wrapping_add(step);
                }
            } else {
                while i > limit {
                    out.push(Value::Int(i));
                    i = i.wrapping_add(step);
                }
            }
            return Ok(Some(out));
        }
        Ok(None)
    }

    /// The instance of the companion object of the std class `name`.
    fn companion_of(&mut self, name: &'static str) -> R<Option<Value>> {
        let Some(c) = self.known_class(name) else { return Ok(None) };
        let Some(companion) = self.syms().class(c).companion else { return Ok(None) };
        Ok(Some(self.module(companion)?))
    }

    /// A thunk or function value, which a by-name argument arrives as.
    fn callable(v: &Value) -> bool {
        matches!(v, Value::Fun(_) | Value::Obj(_))
    }

    #[inline]
    fn call1(&mut self, f: &Value, x: Value) -> R {
        if let Value::Fun(c) = f {
            if let ClosureKind::Lambda(params, body) = &c.kind {
                if params.len == 1 {
                    return self.call_lambda(c, *params, *body, x, None);
                }
            }
        }
        let mut args = self.take_vec(1);
        args.push(x);
        self.apply_value(f.clone(), args)
    }

    #[inline]
    fn call2(&mut self, f: &Value, x: Value, y: Value) -> R {
        if let Value::Fun(c) = f {
            if let ClosureKind::Lambda(params, body) = &c.kind {
                if params.len == 2 {
                    return self.call_lambda(c, *params, *body, x, Some(y));
                }
            }
        }
        let mut args = self.take_vec(2);
        args.push(x);
        args.push(y);
        self.apply_value(f.clone(), args)
    }

    /// A lambda of one or two parameters called from a native, its arguments bound without an
    /// argument vector: what `call_closure` does for a `Lambda`.
    fn call_lambda(&mut self, c: &Rc<Closure>, params: crate::ast::ListRef, body: TExprId, x: Value, y: Option<Value>) -> R {
        self.enter()?;
        self.prof_alloc(|a| a.frames += 1);
        let frame = self.take_frame(Some(c.env.clone()), 2);
        let p = self.prog().sym_lists[params.start as usize];
        frame.bind(p, if matches!(x, Value::Absent) { Value::Unit } else { x });
        if let Some(y) = y {
            let q = self.prog().sym_lists[params.start as usize + 1];
            frame.bind(q, if matches!(y, Value::Absent) { Value::Unit } else { y });
        }
        let cx = Ctx { this: c.this.clone(), def_key: c.def_key, class: c.class, tail: None };
        let prof = self.prof_lambda(body);
        let r = self.eval(body, &frame, &cx);
        self.prof_exit(prof);
        self.release_frame(frame);
        self.depth -= 1;
        r
    }

    fn truth(&mut self, v: Value) -> R<bool> {
        match v {
            Value::Bool(b) => Ok(b),
            other => {
                let shown = self.to_str(&other)?;
                self.unsupported(format!("a condition that is no Boolean: {}", shown))
            }
        }
    }

    /// `pf.applyOrElse(x, miss)`: the value, or `Absent` when no case matched.
    fn apply_or_else(&mut self, pf: &Value, x: Value) -> R {
        let miss = self.partial_miss();
        self.call_by_name(pf.clone(), "applyOrElse", vec![x, miss])
    }

    /// `this.buildC(items)` or `buildCC`; a `Range` builds a `Vector` (`Range.buildCC` is
    /// `Vector.wrap`, through `IterableOps.buildC`), which is made here without the two calls.
    fn build(&mut self, this: &Value, method: &'static str, items: Vec<Value>) -> R {
        if let Value::Obj(o) = this {
            if self.range_of(o).is_some() {
                if let Some(v) = self.flat_vector(items.clone())? {
                    return Ok(v);
                }
            }
        }
        self.call_by_name(this.clone(), method, vec![Value::array(items)])
    }

    /// The constructor arguments of a flat `Vector` over `items`: no root, the items as its
    /// tail, and no chunked twin yet.
    fn flat_vector_args(items: Vec<Value>) -> Vec<Value> {
        let length = items.len() as i32;
        vec![Value::Null, Value::Int(0), Value::Int(0), Value::Int(0), Value::array(items), Value::Int(length), Value::Null]
    }

    /// A flat `Vector` over `items`.
    fn flat_vector(&mut self, items: Vec<Value>) -> R<Option<Value>> {
        let Some(shapes) = self.store_shapes() else { return Ok(None) };
        Ok(Some(self.construct_plain(shapes.vector.0, &shapes.vector.1, Self::flat_vector_args(items))))
    }

    fn hmap_of(&mut self, items: Vec<Value>, pairs: bool) -> R<Option<Rc<MapCell>>> {
        let raw = MapCell::new(HMap::default());
        if pairs {
            let mut entries = Vec::with_capacity(items.len());
            for x in &items {
                let Some(pair) = self.tuple2_parts(x) else { return Ok(None) };
                entries.push(pair);
            }
            for (k, v) in entries {
                self.map_set(&raw, k, v)?;
            }
        } else {
            for x in items {
                self.map_set(&raw, x, Value::Bool(true))?;
            }
        }
        Ok(Some(raw))
    }
}

// ---- List ----

fn list_cons(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some(shape) = it.list_shape() else { return Ok(None) };
    match this {
        Value::Obj(o) if o.class == shape.cons || Rc::ptr_eq(o, &shape.nil) => Ok(Some(it.cons(&shape, arg(a, 0), this.clone()))),
        _ => Ok(None),
    }
}

fn with_list(it: &mut Interp, this: &Value) -> Option<(Rc<ListShape>, Vec<Value>)> {
    let shape = it.list_shape()?;
    let items = it.list_vec(&shape, this)?;
    Some((shape, items))
}

fn list_map(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some((shape, items)) = with_list(it, this) else { return Ok(None) };
    let f = arg(a, 0);
    let mut out = Vec::with_capacity(items.len());
    for x in items {
        out.push(it.call1(&f, x)?);
    }
    Ok(Some(it.vec_list(&shape, out)))
}

fn list_filter(it: &mut Interp, this: &Value, a: &[Value], negate: bool) -> R<Option<Value>> {
    let Some((shape, items)) = with_list(it, this) else { return Ok(None) };
    let p = arg(a, 0);
    let mut out = Vec::new();
    for x in items {
        let keep = it.call1(&p, x.clone())?;
        if it.truth(keep)? != negate {
            out.push(x);
        }
    }
    Ok(Some(it.vec_list(&shape, out)))
}

fn list_flat_map(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some((shape, items)) = with_list(it, this) else { return Ok(None) };
    let f = arg(a, 0);
    let mut out = Vec::new();
    for x in items {
        let r = it.call1(&f, x)?;
        out.extend(it.elements_of(&r)?);
    }
    Ok(Some(it.vec_list(&shape, out)))
}

fn list_foreach(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some((_, items)) = with_list(it, this) else { return Ok(None) };
    let f = arg(a, 0);
    for x in items {
        it.call1(&f, x)?;
    }
    Ok(Some(Value::Unit))
}

fn list_fold_left(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some((_, items)) = with_list(it, this) else { return Ok(None) };
    let (mut acc, op) = (arg(a, 0), arg(a, 1));
    for x in items {
        acc = it.call2(&op, acc, x)?;
    }
    Ok(Some(acc))
}

fn list_exists(it: &mut Interp, this: &Value, a: &[Value], forall: bool) -> R<Option<Value>> {
    let Some((_, items)) = with_list(it, this) else { return Ok(None) };
    let p = arg(a, 0);
    for x in items {
        let r = it.call1(&p, x)?;
        if it.truth(r)? != forall {
            return Ok(Some(Value::Bool(!forall)));
        }
    }
    Ok(Some(Value::Bool(forall)))
}

fn list_find(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some((_, items)) = with_list(it, this) else { return Ok(None) };
    let Some(option) = it.option_shape() else { return Ok(None) };
    let p = arg(a, 0);
    for x in items {
        let r = it.call1(&p, x.clone())?;
        if it.truth(r)? {
            return Ok(Some(it.option_of(&option, Some(x))));
        }
    }
    Ok(Some(it.option_of(&option, None)))
}

fn list_collect_first(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some((_, items)) = with_list(it, this) else { return Ok(None) };
    let Some(option) = it.option_shape() else { return Ok(None) };
    let pf = arg(a, 0);
    for x in items {
        let r = it.apply_or_else(&pf, x)?;
        if !matches!(r, Value::Absent) {
            return Ok(Some(it.option_of(&option, Some(r))));
        }
    }
    Ok(Some(it.option_of(&option, None)))
}

fn list_contains(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some((_, items)) = with_list(it, this) else { return Ok(None) };
    let elem = arg(a, 0);
    for x in items {
        if it.equal(&x, &elem)? {
            return Ok(Some(Value::Bool(true)));
        }
    }
    Ok(Some(Value::Bool(false)))
}

fn list_length(it: &mut Interp, this: &Value, _a: &[Value]) -> R<Option<Value>> {
    let Some((_, items)) = with_list(it, this) else { return Ok(None) };
    Ok(Some(Value::Int(items.len() as i32)))
}

fn list_reverse(it: &mut Interp, this: &Value, _a: &[Value]) -> R<Option<Value>> {
    let Some((shape, mut items)) = with_list(it, this) else { return Ok(None) };
    items.reverse();
    Ok(Some(it.vec_list(&shape, items)))
}

fn list_take(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some((shape, mut items)) = with_list(it, this) else { return Ok(None) };
    let n = arg(a, 0).as_i32().unwrap_or(0).max(0) as usize;
    items.truncate(n);
    Ok(Some(it.vec_list(&shape, items)))
}

fn list_drop(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some(shape) = it.list_shape() else { return Ok(None) };
    let mut n = arg(a, 0).as_i32().unwrap_or(0);
    let mut cur = this.clone();
    while n > 0 {
        let Value::Obj(o) = &cur else { return Ok(None) };
        if Rc::ptr_eq(o, &shape.nil) {
            break;
        }
        if o.class != shape.cons {
            return Ok(None);
        }
        let next = o.fields.borrow().get(shape.slots[1] as usize).cloned();
        match next {
            Some(t) if !matches!(t, Value::Absent) => cur = t,
            _ => return Ok(None),
        }
        n -= 1;
    }
    Ok(Some(cur))
}

fn list_prepend_all(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some(shape) = it.list_shape() else { return Ok(None) };
    let prefix = arg(a, 0);
    let Some(items) = it.list_vec(&shape, &prefix) else { return Ok(None) };
    if it.list_vec(&shape, this).is_none() {
        return Ok(None);
    }
    Ok(Some(it.vec_list_onto(&shape, items, this.clone())))
}

fn list_head_option(it: &mut Interp, this: &Value, _a: &[Value]) -> R<Option<Value>> {
    let Some(shape) = it.list_shape() else { return Ok(None) };
    let Some(option) = it.option_shape() else { return Ok(None) };
    let Value::Obj(o) = this else { return Ok(None) };
    if Rc::ptr_eq(o, &shape.nil) {
        return Ok(Some(it.option_of(&option, None)));
    }
    if o.class != shape.cons {
        return Ok(None);
    }
    let head = o.fields.borrow().get(shape.slots[0] as usize).cloned();
    match head {
        Some(h) if !matches!(h, Value::Absent) => Ok(Some(it.option_of(&option, Some(h)))),
        _ => Ok(None),
    }
}

fn list_zip_with_index(it: &mut Interp, this: &Value, _a: &[Value]) -> R<Option<Value>> {
    let Some((shape, items)) = with_list(it, this) else { return Ok(None) };
    let mut out = Vec::with_capacity(items.len());
    for (i, x) in items.into_iter().enumerate() {
        let Some(pair) = it.tuple2(x, Value::Int(i as i32)) else { return Ok(None) };
        out.push(pair);
    }
    Ok(Some(it.vec_list(&shape, out)))
}

fn from_array(it: &mut Interp, _this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some(shape) = it.list_shape() else { return Ok(None) };
    let Value::Array(arr) = arg(a, 0) else { return Ok(None) };
    let items = arr.borrow().clone();
    Ok(Some(it.vec_list(&shape, items)))
}

fn iterable_to_array(it: &mut Interp, _this: &Value, a: &[Value]) -> R<Option<Value>> {
    let xs = arg(a, 0);
    match it.elements_fast(&xs)? {
        Some(items) => Ok(Some(Value::array(items))),
        None => Ok(None),
    }
}

fn join_strings(it: &mut Interp, _this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some(items) = it.elements_fast(&arg(a, 0))? else { return Ok(None) };
    if items.iter().any(|x| matches!(x, Value::Null)) {
        return Ok(None);
    }
    let (start, sep, end) = (arg(a, 1), arg(a, 2), arg(a, 3));
    let mut s = it.to_str(&start)?;
    let sep = it.to_str(&sep)?;
    for (i, x) in items.iter().enumerate() {
        if i > 0 {
            crate::text::push_str(&mut s, &sep);
        }
        let text = it.to_str(x)?;
        crate::text::push_str(&mut s, &text);
    }
    crate::text::push_str(&mut s, &it.to_str(&end)?);
    Ok(Some(Value::string(s)))
}

// ---- Array and String extensions: the receiver is the first argument ----

fn array_items(a: &[Value]) -> Option<Vec<Value>> {
    match a.first() {
        Some(Value::Array(arr)) => Some(arr.borrow().clone()),
        _ => None,
    }
}

fn array_foreach(it: &mut Interp, _this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some(items) = array_items(a) else { return Ok(None) };
    let f = arg(a, 1);
    for x in items {
        it.call1(&f, x)?;
    }
    Ok(Some(Value::Unit))
}

fn array_fold_left(it: &mut Interp, _this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some(items) = array_items(a) else { return Ok(None) };
    let (mut acc, op) = (arg(a, 1), arg(a, 2));
    for x in items {
        acc = it.call2(&op, acc, x)?;
    }
    Ok(Some(acc))
}

fn array_find(it: &mut Interp, _this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some(items) = array_items(a) else { return Ok(None) };
    let Some(option) = it.option_shape() else { return Ok(None) };
    let p = arg(a, 1);
    for x in items {
        let r = it.call1(&p, x.clone())?;
        if it.truth(r)? {
            return Ok(Some(it.option_of(&option, Some(x))));
        }
    }
    Ok(Some(it.option_of(&option, None)))
}

fn array_exists(it: &mut Interp, _this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some(items) = array_items(a) else { return Ok(None) };
    let p = arg(a, 1);
    for x in items {
        let r = it.call1(&p, x)?;
        if it.truth(r)? {
            return Ok(Some(Value::Bool(true)));
        }
    }
    Ok(Some(Value::Bool(false)))
}

fn array_to_list(it: &mut Interp, _this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some(shape) = it.list_shape() else { return Ok(None) };
    let Some(items) = array_items(a) else { return Ok(None) };
    Ok(Some(it.vec_list(&shape, items)))
}

fn string_fold_left(it: &mut Interp, _this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some(Value::Str(s)) = a.first() else { return Ok(None) };
    let (mut acc, op) = (arg(a, 1), arg(a, 2));
    for c in crate::text::utf16_units(s).collect::<Vec<u16>>() {
        acc = it.call2(&op, acc, Value::Char(c))?;
    }
    Ok(Some(acc))
}

// ---- Option ----

fn with_option(it: &mut Interp, this: &Value) -> Option<(Rc<OptionShape>, Option<Value>)> {
    let shape = it.option_shape()?;
    let v = it.option_value(&shape, this)?;
    Some((shape, v))
}

fn option_map(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some((shape, v)) = with_option(it, this) else { return Ok(None) };
    let f = arg(a, 0);
    Ok(Some(match v {
        Some(x) => {
            let y = it.call1(&f, x)?;
            it.option_of(&shape, Some(y))
        }
        None => this.clone(),
    }))
}

fn option_flat_map(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some((_, v)) = with_option(it, this) else { return Ok(None) };
    let f = arg(a, 0);
    Ok(Some(match v {
        Some(x) => it.call1(&f, x)?,
        None => this.clone(),
    }))
}

fn option_get_or_else(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some((_, v)) = with_option(it, this) else { return Ok(None) };
    if !Interp::callable(&arg(a, 0)) {
        return Ok(None);
    }
    Ok(Some(match v {
        Some(x) => x,
        None => it.apply_value(arg(a, 0), Vec::new())?,
    }))
}

fn option_or_else(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some((_, v)) = with_option(it, this) else { return Ok(None) };
    if !Interp::callable(&arg(a, 0)) {
        return Ok(None);
    }
    Ok(Some(match v {
        Some(_) => this.clone(),
        None => it.apply_value(arg(a, 0), Vec::new())?,
    }))
}

fn option_foreach(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some((_, v)) = with_option(it, this) else { return Ok(None) };
    if let Some(x) = v {
        it.call1(&arg(a, 0), x)?;
    }
    Ok(Some(Value::Unit))
}

fn option_test(it: &mut Interp, this: &Value, empty: bool) -> R<Option<Value>> {
    let Some((_, v)) = with_option(it, this) else { return Ok(None) };
    Ok(Some(Value::Bool(v.is_none() == empty)))
}

fn option_exists(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some((_, v)) = with_option(it, this) else { return Ok(None) };
    Ok(Some(match v {
        Some(x) => it.call1(&arg(a, 0), x)?,
        None => Value::Bool(false),
    }))
}

fn option_forall(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some((_, v)) = with_option(it, this) else { return Ok(None) };
    Ok(Some(match v {
        Some(x) => it.call1(&arg(a, 0), x)?,
        None => Value::Bool(true),
    }))
}

fn option_filter(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some((shape, v)) = with_option(it, this) else { return Ok(None) };
    Ok(Some(match v {
        Some(x) => {
            let r = it.call1(&arg(a, 0), x)?;
            if it.truth(r)? { this.clone() } else { it.option_of(&shape, None) }
        }
        None => this.clone(),
    }))
}

fn option_fold(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some((_, v)) = with_option(it, this) else { return Ok(None) };
    if !Interp::callable(&arg(a, 0)) {
        return Ok(None);
    }
    Ok(Some(match v {
        Some(x) => it.call1(&arg(a, 1), x)?,
        None => it.apply_value(arg(a, 0), Vec::new())?,
    }))
}

fn option_to_list(it: &mut Interp, this: &Value, _a: &[Value]) -> R<Option<Value>> {
    let Some((_, v)) = with_option(it, this) else { return Ok(None) };
    let Some(shape) = it.list_shape() else { return Ok(None) };
    Ok(Some(it.vec_list(&shape, v.into_iter().collect())))
}

// ---- Range and the array-backed sequences ----

fn range_foreach(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Value::Obj(o) = this else { return Ok(None) };
    let Some((start, limit, step)) = it.range_of(o) else { return Ok(None) };
    let f = arg(a, 0);
    let mut i = start;
    if step > 0 {
        while i < limit {
            it.call1(&f, Value::Int(i))?;
            i = i.wrapping_add(step);
        }
    } else {
        while i > limit {
            it.call1(&f, Value::Int(i))?;
            i = i.wrapping_add(step);
        }
    }
    Ok(Some(Value::Unit))
}

fn range_length(it: &mut Interp, this: &Value, _a: &[Value]) -> R<Option<Value>> {
    let Value::Obj(o) = this else { return Ok(None) };
    let Some((start, limit, step)) = it.range_of(o) else { return Ok(None) };
    if step == 0 {
        return Ok(None);
    }
    let empty = if step > 0 { start >= limit } else { start <= limit };
    let n = if empty {
        0
    } else if step > 0 {
        (limit as i64 - start as i64 + step as i64 - 1) / step as i64
    } else {
        (start as i64 - limit as i64 - step as i64 - 1) / (-(step as i64))
    };
    Ok(Some(Value::Int(n as i32)))
}

fn range_is_empty(it: &mut Interp, this: &Value, _a: &[Value]) -> R<Option<Value>> {
    let Value::Obj(o) = this else { return Ok(None) };
    let Some((start, limit, step)) = it.range_of(o) else { return Ok(None) };
    Ok(Some(Value::Bool(if step > 0 { start >= limit } else { start <= limit })))
}

fn items_foreach(it: &mut Interp, this: &Value, a: &[Value], class: &'static str) -> R<Option<Value>> {
    let Value::Obj(o) = this else { return Ok(None) };
    let Some(items) = it.items_of(o, class) else { return Ok(None) };
    let f = arg(a, 0);
    for x in items {
        it.call1(&f, x)?;
    }
    Ok(Some(Value::Unit))
}

fn items_to_list(it: &mut Interp, this: &Value, class: &'static str) -> R<Option<Value>> {
    let Value::Obj(o) = this else { return Ok(None) };
    let Some(shape) = it.list_shape() else { return Ok(None) };
    let Some(items) = it.items_of(o, class) else { return Ok(None) };
    Ok(Some(it.vec_list(&shape, items)))
}

fn vector_map(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Value::Obj(o) = this else { return Ok(None) };
    let Some(items) = it.items_of(o, "Vector") else { return Ok(None) };
    let Some(companion) = it.companion_of("Vector")? else { return Ok(None) };
    let f = arg(a, 0);
    let mut out = Vec::with_capacity(items.len());
    for x in items {
        out.push(it.call1(&f, x)?);
    }
    Ok(Some(it.call_by_name(companion, "wrap", vec![Value::array(out)])?))
}

// ---- sets and maps ----

/// `PersistentMap.ordinalOf(key)` and `HashSet.ordinalOf(elem)`: the ordinal of the key's
/// entry, or -1, by a scan of a small store's entries or a walk of its trie index, as the
/// bodies do (std/maps.scala); `pairs` says whether an entry is a tuple whose first part is
/// the key. Answers `None` for an entry vector that is a trie.
/// The plan of `PersistentMap` (index, entries, dead, fallback, compacted) or `HashSet` (index,
/// entries, dead, compacted), when `o` is one.
fn store_shape(it: &mut Interp, o: &Rc<Object>, pairs: bool) -> Option<(ClassId, Rc<[u32]>)> {
    let shapes = it.store_shapes()?;
    let plan = if pairs { &shapes.map } else { &shapes.set };
    if o.class != plan.0 { None } else { Some(plan.clone()) }
}

fn store_ordinal(it: &mut Interp, this: &Value, a: &[Value], pairs: bool) -> R<Option<Value>> {
    let Value::Obj(o) = this else { return Ok(None) };
    let Some((_, slots)) = store_shape(it, o, pairs) else { return Ok(None) };
    let key = arg(a, 0);
    match Interp::slot_value(o, slots[0]) {
        Some(Value::Null) => {
            let Some(Value::Obj(entries)) = Interp::slot_value(o, slots[1]) else { return Ok(None) };
            let mut i = 0;
            while let Some(e) = it.vector_item(&entries, i) {
                let k = if pairs {
                    let Some((k, _)) = it.tuple2_parts(&e) else { return Ok(None) };
                    k
                } else {
                    e
                };
                if it.equal(&k, &key)? {
                    return Ok(Some(Value::Int(i)));
                }
                i += 1;
            }
            Ok(Some(Value::Int(-1)))
        }
        Some(index) => {
            let hash = it.hash(&key)?;
            trie_find(it, &index, &key, hash, 0)
        }
        None => Ok(None),
    }
}

/// The store's entries when it has no tombstones and its entry vector is flat.
fn store_entries(it: &mut Interp, this: &Value) -> Option<Vec<Value>> {
    let Value::Obj(o) = this else { return None };
    let (_, slots) = store_shape(it, o, true).or_else(|| store_shape(it, o, false))?;
    if !matches!(Interp::slot_value(o, slots[2]), Some(Value::Int(0))) {
        return None;
    }
    let Some(Value::Obj(entries)) = Interp::slot_value(o, slots[1]) else { return None };
    it.items_of(&entries, "Vector")
}

/// The value of the map's entry at `ordinal`.
fn map_value_at(it: &mut Interp, this: &Value, ordinal: i32) -> Option<Value> {
    let Value::Obj(o) = this else { return None };
    let (_, slots) = store_shape(it, o, true)?;
    let Some(Value::Obj(entries)) = Interp::slot_value(o, slots[1]) else { return None };
    let entry = it.vector_item(&entries, ordinal)?;
    let (_, v) = it.tuple2_parts(&entry)?;
    Some(v)
}

fn store_contains(it: &mut Interp, this: &Value, a: &[Value], pairs: bool) -> R<Option<Value>> {
    let Some(Value::Int(i)) = store_ordinal(it, this, a, pairs)? else { return Ok(None) };
    Ok(Some(Value::Bool(i >= 0)))
}

fn map_get(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some(option) = it.option_shape() else { return Ok(None) };
    let Some(Value::Int(i)) = store_ordinal(it, this, a, true)? else { return Ok(None) };
    if i < 0 {
        return Ok(Some(it.option_of(&option, None)));
    }
    let Some(v) = map_value_at(it, this, i) else { return Ok(None) };
    Ok(Some(it.option_of(&option, Some(v))))
}

fn map_get_or_else(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    if !Interp::callable(&arg(a, 1)) {
        return Ok(None);
    }
    let Some(Value::Int(i)) = store_ordinal(it, this, a, true)? else { return Ok(None) };
    if i >= 0 {
        let Some(v) = map_value_at(it, this, i) else { return Ok(None) };
        return Ok(Some(v));
    }
    Ok(Some(it.apply_value(arg(a, 1), Vec::new())?))
}

fn store_to_array(it: &mut Interp, this: &Value, _a: &[Value]) -> R<Option<Value>> {
    Ok(store_entries(it, this).map(Value::array))
}

/// `PersistentMap.keys` and `values`: the entries' first or second parts as a `List`.
fn map_parts(it: &mut Interp, this: &Value, part: usize) -> R<Option<Value>> {
    let Some(shape) = it.list_shape() else { return Ok(None) };
    let Some(entries) = store_entries(it, this) else { return Ok(None) };
    let mut out = Vec::with_capacity(entries.len());
    for e in &entries {
        let Some((k, v)) = it.tuple2_parts(e) else { return Ok(None) };
        out.push(if part == 0 { k } else { v });
    }
    Ok(Some(it.vec_list(&shape, out)))
}

/// `Vector.apply(i)` of a flat vector, in range.
fn vector_apply(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let (Value::Obj(o), Value::Int(i)) = (this, arg(a, 0)) else { return Ok(None) };
    Ok(it.vector_item(o, i))
}

// ---- the hash trie index, native ----
//
// The natives of `trieFind`, `trieInsert`, `trieRemove`, `trieOf` and `trieRebuilt` keep the
// index as `Value::Trie` nodes, whose content holds the triples (key, hash, ordinal) and the
// sub-nodes as `std/maps.scala` lays them out; a Scala `TrieNode` object reaches them only when
// some of them are off (`TEQ_NO_NATIVES=trie` switches them off together), and `trieFind`
// walks such a node's fields, the others leave it to the Scala body.

fn trie_bit(hash: i32, shift: u32) -> i32 {
    1i32 << (((hash as u32) >> shift) & 31)
}

fn trie_rank(bitmap: i32, bit: i32) -> usize {
    (bitmap & bit.wrapping_sub(1)).count_ones() as usize
}

fn trie_value(data_map: i32, node_map: i32, content: Vec<Value>) -> Value {
    Value::Trie(Rc::new(Trie::new(data_map, node_map, content)))
}

fn same_hash(v: &Value, hash: i32) -> bool {
    matches!(v, Value::Int(h) if *h == hash)
}

/// `trieFind(node, key, hash, shift)`: the ordinal or -1.
fn trie_find_native(it: &mut Interp, _this: &Value, a: &[Value]) -> R<Option<Value>> {
    let (Value::Int(hash), Value::Int(shift)) = (arg(a, 2), arg(a, 3)) else { return Ok(None) };
    trie_find(it, &arg(a, 0), &arg(a, 1), hash, shift as u32)
}

fn trie_find(it: &mut Interp, node: &Value, key: &Value, hash: i32, shift: u32) -> R<Option<Value>> {
    match node {
        Value::Trie(t) => {
            let t = t.clone();
            if shift > 30 {
                let mut i = 0;
                while i + 2 < t.content.len() {
                    if same_hash(&t.content[i + 1], hash) && it.equal(&t.content[i], key)? {
                        return Ok(Some(t.content[i + 2].clone()));
                    }
                    i += 3;
                }
                return Ok(Some(Value::Int(-1)));
            }
            let bit = trie_bit(hash, shift);
            if t.data_map & bit != 0 {
                let i = 3 * trie_rank(t.data_map, bit);
                let found = same_hash(&t.content[i + 1], hash) && it.equal(&t.content[i], key)?;
                return Ok(Some(if found { t.content[i + 2].clone() } else { Value::Int(-1) }));
            }
            if t.node_map & bit != 0 {
                let j = t.content.len() - 1 - trie_rank(t.node_map, bit);
                return trie_find(it, &t.content[j], key, hash, shift + 5);
            }
            Ok(Some(Value::Int(-1)))
        }
        Value::Obj(o) => {
            let Some(shapes) = it.store_shapes() else { return Ok(None) };
            let plan = shapes.trie.clone();
            trie_find_object(it, o, key, hash, shift, &plan)
        }
        _ => Ok(None),
    }
}

/// `trieFind` over a Scala `TrieNode` object (dataMap, nodeMap, content).
fn trie_find_object(it: &mut Interp, node: &Rc<Object>, key: &Value, hash: i32, shift: u32, plan: &(ClassId, Rc<[u32]>)) -> R<Option<Value>> {
    if node.class != plan.0 {
        return Ok(None);
    }
    let (Some(Value::Int(data_map)), Some(Value::Int(node_map)), Some(Value::Array(content))) =
        (Interp::slot_value(node, plan.1[0]), Interp::slot_value(node, plan.1[1]), Interp::slot_value(node, plan.1[2]))
    else {
        return Ok(None);
    };
    let content = content.borrow().clone();
    if shift > 30 {
        let mut i = 0;
        while i + 2 < content.len() {
            if same_hash(&content[i + 1], hash) && it.equal(&content[i], key)? {
                return Ok(Some(content[i + 2].clone()));
            }
            i += 3;
        }
        return Ok(Some(Value::Int(-1)));
    }
    let bit = trie_bit(hash, shift);
    if data_map & bit != 0 {
        let i = 3 * trie_rank(data_map, bit);
        let (Some(k), Some(h), Some(ordinal)) = (content.get(i), content.get(i + 1), content.get(i + 2)) else { return Ok(None) };
        let found = same_hash(h, hash) && it.equal(k, key)?;
        return Ok(Some(if found { ordinal.clone() } else { Value::Int(-1) }));
    }
    if node_map & bit != 0 {
        let j = content.len() - 1 - trie_rank(node_map, bit);
        return trie_find(it, &content[j], key, hash, shift + 5);
    }
    Ok(Some(Value::Int(-1)))
}

/// `trieInsert(node, key, hash, shift, ordinal)`: the node with the key it does not hold, one
/// path copied.
fn trie_insert_native(_it: &mut Interp, _this: &Value, a: &[Value]) -> R<Option<Value>> {
    let (Value::Trie(node), Value::Int(hash), Value::Int(shift), Value::Int(ordinal)) = (arg(a, 0), arg(a, 2), arg(a, 3), arg(a, 4)) else { return Ok(None) };
    let mut node = node;
    trie_insert(&mut node, &arg(a, 1), hash, shift as u32, ordinal);
    Ok(Some(Value::Trie(node)))
}

/// Inserts into `node`, in place when this builder alone holds it (`Rc::make_mut`), through a
/// copy of the path otherwise: the persistent and the transient insertion in one. No key's
/// `hashCode` or `equals` runs: the hashes are stored.
fn trie_insert(node: &mut Rc<Trie>, key: &Value, hash: i32, shift: u32, ordinal: i32) {
    if shift > 30 {
        let t = Rc::make_mut(node);
        t.content.push(key.clone());
        t.content.push(Value::Int(hash));
        t.content.push(Value::Int(ordinal));
        return;
    }
    let bit = trie_bit(hash, shift);
    if node.data_map & bit != 0 {
        let i = 3 * trie_rank(node.data_map, bit);
        let (k0, Value::Int(h0), Value::Int(o0)) = (node.content[i].clone(), node.content[i + 1].clone(), node.content[i + 2].clone()) else { return };
        let sub = trie_merge(k0, h0, o0, key.clone(), hash, ordinal, shift + 5);
        let t = Rc::make_mut(node);
        let at = 3 * (t.data_map.count_ones() as usize - 1) + t.node_map.count_ones() as usize - trie_rank(t.node_map, bit);
        t.content.drain(i..i + 3);
        t.content.insert(at, sub);
        t.data_map ^= bit;
        t.node_map |= bit;
        return;
    }
    if node.node_map & bit != 0 {
        let j = node.content.len() - 1 - trie_rank(node.node_map, bit);
        let t = Rc::make_mut(node);
        let Value::Trie(sub) = &mut t.content[j] else { return };
        trie_insert(sub, key, hash, shift + 5, ordinal);
        return;
    }
    let i = 3 * trie_rank(node.data_map, bit);
    let t = Rc::make_mut(node);
    t.content.insert(i, Value::Int(ordinal));
    t.content.insert(i, Value::Int(hash));
    t.content.insert(i, key.clone());
    t.data_map |= bit;
}

fn trie_merge(k0: Value, h0: i32, o0: i32, k1: Value, h1: i32, o1: i32, shift: u32) -> Value {
    if shift > 30 {
        return trie_value(0, 0, vec![k0, Value::Int(h0), Value::Int(o0), k1, Value::Int(h1), Value::Int(o1)]);
    }
    let b0 = trie_bit(h0, shift);
    let b1 = trie_bit(h1, shift);
    if b0 == b1 {
        let sub = trie_merge(k0, h0, o0, k1, h1, o1, shift + 5);
        return trie_value(0, b0, vec![sub]);
    }
    let triples = if b0 & b1.wrapping_sub(1) != 0 {
        vec![k0, Value::Int(h0), Value::Int(o0), k1, Value::Int(h1), Value::Int(o1)]
    } else {
        vec![k1, Value::Int(h1), Value::Int(o1), k0, Value::Int(h0), Value::Int(o0)]
    };
    trie_value(b0 | b1, 0, triples)
}

/// `trieRemove(node, key, hash, shift)`: the node without the key it holds; a sub-node left
/// with one triple is inlined into its parent, one left empty is dropped.
fn trie_remove_native(it: &mut Interp, _this: &Value, a: &[Value]) -> R<Option<Value>> {
    let (Value::Trie(node), Value::Int(hash), Value::Int(shift)) = (arg(a, 0), arg(a, 2), arg(a, 3)) else { return Ok(None) };
    Ok(Some(Value::Trie(trie_remove(it, &node, &arg(a, 1), hash, shift as u32)?)))
}

fn trie_remove(it: &mut Interp, node: &Rc<Trie>, key: &Value, hash: i32, shift: u32) -> R<Rc<Trie>> {
    let mut content = node.content.clone();
    if shift > 30 {
        let mut i = 0;
        while i + 2 < content.len() && !(same_hash(&content[i + 1], hash) && it.equal(&content[i], key)?) {
            i += 3;
        }
        if i + 2 < content.len() {
            content.drain(i..i + 3);
        }
        return Ok(Rc::new(Trie::new(0, 0, content)));
    }
    let bit = trie_bit(hash, shift);
    if node.data_map & bit != 0 {
        let i = 3 * trie_rank(node.data_map, bit);
        content.drain(i..i + 3);
        return Ok(Rc::new(Trie::new(node.data_map ^ bit, node.node_map, content)));
    }
    let j = content.len() - 1 - trie_rank(node.node_map, bit);
    let Value::Trie(sub) = &content[j] else { return Ok(node.clone()) };
    let sub = trie_remove(it, &sub.clone(), key, hash, shift + 5)?;
    if sub.content.is_empty() {
        content.remove(j);
        return Ok(Rc::new(Trie::new(node.data_map, node.node_map ^ bit, content)));
    }
    if sub.node_map == 0 && sub.content.len() == 3 {
        content.remove(j);
        let i = 3 * trie_rank(node.data_map, bit);
        content.insert(i, sub.content[2].clone());
        content.insert(i, sub.content[1].clone());
        content.insert(i, sub.content[0].clone());
        return Ok(Rc::new(Trie::new(node.data_map | bit, node.node_map ^ bit, content)));
    }
    content[j] = Value::Trie(sub);
    Ok(Rc::new(Trie::new(node.data_map, node.node_map, content)))
}

/// `trieOf(entries, pairs)`: the index over the entries, or their first parts, built in place.
fn trie_of_native(it: &mut Interp, _this: &Value, a: &[Value]) -> R<Option<Value>> {
    let (Value::Array(entries), Value::Bool(pairs)) = (arg(a, 0), arg(a, 1)) else { return Ok(None) };
    let entries = entries.borrow().clone();
    trie_of(it, &entries, pairs)
}

fn trie_of(it: &mut Interp, entries: &[Value], pairs: bool) -> R<Option<Value>> {
    let mut root = Rc::new(Trie::new(0, 0, Vec::new()));
    for (i, e) in entries.iter().enumerate() {
        let key = if pairs {
            let Some((k, _)) = it.tuple2_parts(e) else { return Ok(None) };
            k
        } else {
            e.clone()
        };
        let hash = it.hash(&key)?;
        trie_insert(&mut root, &key, hash, 0, i as i32);
    }
    Ok(Some(Value::Trie(root)))
}

/// `trieRebuilt(old, ordinals)`: the index with each ordinal replaced by `ordinals(ordinal)`
/// and the entries whose slot holds -1 left out, from the stored hashes.
fn trie_rebuilt_native(_it: &mut Interp, _this: &Value, a: &[Value]) -> R<Option<Value>> {
    let (Value::Trie(old), Value::Array(ordinals)) = (arg(a, 0), arg(a, 1)) else { return Ok(None) };
    let ordinals: Vec<i32> = ordinals.borrow().iter().map(|v| if let Value::Int(i) = v { *i } else { -1 }).collect();
    let mut root = Rc::new(Trie::new(0, 0, Vec::new()));
    trie_copy_into(&mut root, &old, &ordinals);
    Ok(Some(Value::Trie(root)))
}

fn trie_copy_into(root: &mut Rc<Trie>, node: &Rc<Trie>, ordinals: &[i32]) {
    let triples = if node.data_map == 0 && node.node_map == 0 { node.content.len() } else { 3 * node.data_map.count_ones() as usize };
    let mut i = 0;
    while i + 2 < triples + 1 && i + 2 < node.content.len() {
        if let (Value::Int(hash), Value::Int(ordinal)) = (&node.content[i + 1], &node.content[i + 2]) {
            if let Some(&next) = ordinals.get(*ordinal as usize) {
                if next >= 0 {
                    trie_insert(root, &node.content[i], *hash, 0, next);
                }
            }
        }
        i += 3;
    }
    for sub in &node.content[triples..] {
        if let Value::Trie(sub) = sub {
            trie_copy_into(root, sub, ordinals);
        }
    }
}

/// `PersistentMap.fromEntries(entries)` and `HashSet.fromDistinct(elems)`: the store over the
/// entries, indexed past `smallStoreSize`; the shared empty one for none.
fn store_of(it: &mut Interp, a: &[Value], pairs: bool) -> R<Option<Value>> {
    let Value::Array(entries) = arg(a, 0) else { return Ok(None) };
    let items = entries.borrow().clone();
    store_from(it, items, pairs)
}

fn store_from(it: &mut Interp, items: Vec<Value>, pairs: bool) -> R<Option<Value>> {
    // With the trie natives off (`TEQ_NO_NATIVES=trie`) the index must be the Scala body's
    // objects, which the Scala trie operations read: the bulk constructors leave the building
    // to their bodies then, as one group with the trie's operations.
    if native_off("scala.trieOf") {
        return Ok(None);
    }
    let Some(shapes) = it.store_shapes() else { return Ok(None) };
    let (store, slots) = if pairs { shapes.map.clone() } else { shapes.set.clone() };
    if items.is_empty() {
        let Some(companion) = it.companion_of(if pairs { "Map" } else { "Set" })? else { return Ok(None) };
        return Ok(Some(it.call_by_name(companion, "empty", Vec::new())?));
    }
    let index = if items.len() <= 8 {
        Value::Null
    } else {
        let Some(index) = trie_of(it, &items, pairs)? else { return Ok(None) };
        index
    };
    let Some(entries) = it.flat_vector(items)? else { return Ok(None) };
    let mut args = vec![index, entries, Value::Int(0)];
    if pairs {
        let Some(option) = it.option_shape() else { return Ok(None) };
        args.push(it.option_of(&option, None));
    }
    args.push(Value::Null);
    Ok(Some(it.construct_plain(store, &slots, args)))
}

/// `Map.fromRaw(raw)`: the map over the store's entries as pairs.
fn map_from_raw(it: &mut Interp, _this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Value::Map(raw) = arg(a, 0) else { return Ok(None) };
    map_of_raw(it, &raw)
}

fn map_of_raw(it: &mut Interp, raw: &Rc<MapCell>) -> R<Option<Value>> {
    let mut pairs = Vec::with_capacity(Interp::map_size(raw));
    for (k, v) in Interp::map_entries(raw) {
        let Some(pair) = it.tuple2(k, v) else { return Ok(None) };
        pairs.push(pair);
    }
    store_from(it, pairs, true)
}

/// `PersistentMap.foreach(f)` and `HashSet.foreach(f)` over a store without tombstones whose
/// entry vector is flat.
fn store_foreach(it: &mut Interp, this: &Value, a: &[Value], pairs: bool) -> R<Option<Value>> {
    let Value::Obj(o) = this else { return Ok(None) };
    if !matches!(it.field_of(o, "dead"), Some(Value::Int(0))) {
        return Ok(None);
    }
    let Some(Value::Obj(entries)) = it.field_of(o, "entries") else { return Ok(None) };
    let Some(items) = it.items_of(&entries, "Vector") else { return Ok(None) };
    if pairs && items.iter().any(|e| it.tuple2_parts(e).is_none()) {
        return Ok(None);
    }
    let f = arg(a, 0);
    for e in items {
        it.call1(&f, e)?;
    }
    Ok(Some(Value::Unit))
}

/// `Set.from(elems)`: the distinct elements, in their order, through `HashSet.fromDistinct`;
/// the set itself for a `HashSet`.
fn set_of_elements(it: &mut Interp, elems: &Value) -> R<Option<Value>> {
    if let Value::Obj(o) = elems {
        if it.known_class("HashSet") == Some(o.class) {
            return Ok(Some(elems.clone()));
        }
    }
    let Some(companion) = it.companion_of("HashSet")? else { return Ok(None) };
    let Some(items) = it.elements_fast(elems)? else { return Ok(None) };
    let Some(raw) = it.hmap_of(items, false)? else { return Ok(None) };
    let keys = Interp::map_keys(&raw);
    Ok(Some(it.call_by_name(companion, "fromDistinct", vec![Value::array(keys)])?))
}

fn set_from(it: &mut Interp, _this: &Value, a: &[Value]) -> R<Option<Value>> {
    set_of_elements(it, &arg(a, 0))
}

/// `Map.from(entries)`: the entries collected in a raw store, through `Map.fromRaw`; the map
/// itself for a `Map`.
fn map_of_elements(it: &mut Interp, entries: &Value) -> R<Option<Value>> {
    let Some(map) = it.known_class("Map") else { return Ok(None) };
    if let Value::Obj(o) = entries {
        if it.is_subclass(o.class, map) {
            return Ok(Some(entries.clone()));
        }
    }
    let Some(companion) = it.companion_of("Map")? else { return Ok(None) };
    let Some(items) = it.elements_fast(entries)? else { return Ok(None) };
    let Some(raw) = it.hmap_of(items, true)? else { return Ok(None) };
    match map_of_raw(it, &raw)? {
        Some(m) => Ok(Some(m)),
        None => Ok(Some(it.call_by_name(companion, "fromRaw", vec![Value::Map(raw)])?)),
    }
}

fn map_from(it: &mut Interp, _this: &Value, a: &[Value]) -> R<Option<Value>> {
    map_of_elements(it, &arg(a, 0))
}

// ---- IterableOps over any collection ----

fn iterable_to_list(it: &mut Interp, this: &Value, _a: &[Value]) -> R<Option<Value>> {
    let Some(shape) = it.list_shape() else { return Ok(None) };
    let Some(items) = it.elements_fast(this)? else { return Ok(None) };
    Ok(Some(it.vec_list(&shape, items)))
}

fn iterable_map(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some(items) = it.elements_fast(this)? else { return Ok(None) };
    let f = arg(a, 0);
    let mut out = Vec::with_capacity(items.len());
    for x in items {
        out.push(it.call1(&f, x)?);
    }
    Ok(Some(it.build(this, "buildCC", out)?))
}

fn iterable_flat_map(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some(items) = it.elements_fast(this)? else { return Ok(None) };
    let f = arg(a, 0);
    let mut out = Vec::new();
    for x in items {
        let r = it.call1(&f, x)?;
        out.extend(it.elements_of(&r)?);
    }
    Ok(Some(it.build(this, "buildCC", out)?))
}

fn iterable_filter(it: &mut Interp, this: &Value, a: &[Value], negate: bool) -> R<Option<Value>> {
    let Some(items) = it.elements_fast(this)? else { return Ok(None) };
    let p = arg(a, 0);
    let mut out = Vec::new();
    for x in items {
        let keep = it.call1(&p, x.clone())?;
        if it.truth(keep)? != negate {
            out.push(x);
        }
    }
    Ok(Some(it.build(this, "buildC", out)?))
}

fn iterable_fold_left(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some(items) = it.elements_fast(this)? else { return Ok(None) };
    let (mut acc, op) = (arg(a, 0), arg(a, 1));
    for x in items {
        acc = it.call2(&op, acc, x)?;
    }
    Ok(Some(acc))
}

fn iterable_exists(it: &mut Interp, this: &Value, a: &[Value], forall: bool) -> R<Option<Value>> {
    let Some(items) = it.elements_fast(this)? else { return Ok(None) };
    let p = arg(a, 0);
    for x in items {
        let r = it.call1(&p, x)?;
        if it.truth(r)? != forall {
            return Ok(Some(Value::Bool(!forall)));
        }
    }
    Ok(Some(Value::Bool(forall)))
}

fn iterable_find(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some(items) = it.elements_fast(this)? else { return Ok(None) };
    let Some(option) = it.option_shape() else { return Ok(None) };
    let p = arg(a, 0);
    for x in items {
        let r = it.call1(&p, x.clone())?;
        if it.truth(r)? {
            return Ok(Some(it.option_of(&option, Some(x))));
        }
    }
    Ok(Some(it.option_of(&option, None)))
}

fn iterable_collect_first(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some(items) = it.elements_fast(this)? else { return Ok(None) };
    let Some(option) = it.option_shape() else { return Ok(None) };
    let pf = arg(a, 0);
    for x in items {
        let r = it.apply_or_else(&pf, x)?;
        if !matches!(r, Value::Absent) {
            return Ok(Some(it.option_of(&option, Some(r))));
        }
    }
    Ok(Some(it.option_of(&option, None)))
}

fn iterable_size(it: &mut Interp, this: &Value, _a: &[Value]) -> R<Option<Value>> {
    if let Some(n) = it.known_size(this) {
        return Ok(Some(Value::Int(n as i32)));
    }
    let Some(items) = it.elements_fast(this)? else { return Ok(None) };
    Ok(Some(Value::Int(items.len() as i32)))
}

fn iterable_empty(it: &mut Interp, this: &Value, empty: bool) -> R<Option<Value>> {
    if let Some(n) = it.known_size(this) {
        return Ok(Some(Value::Bool((n == 0) == empty)));
    }
    let Some(items) = it.elements_fast(this)? else { return Ok(None) };
    Ok(Some(Value::Bool(items.is_empty() == empty)))
}

fn iterable_head_option(it: &mut Interp, this: &Value, _a: &[Value]) -> R<Option<Value>> {
    let Some(items) = it.elements_fast(this)? else { return Ok(None) };
    let Some(option) = it.option_shape() else { return Ok(None) };
    Ok(Some(it.option_of(&option, items.into_iter().next())))
}

fn iterable_mk_string(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    if a.len() != 3 {
        return Ok(None);
    }
    let mut all = vec![this.clone()];
    all.extend_from_slice(a);
    join_strings(it, &Value::Unit, &all)
}

fn grouped(it: &mut Interp, this: &Value, key: &Value, f: Option<&Value>) -> R<Option<Rc<MapCell>>> {
    let Some(items) = it.elements_fast(this)? else { return Ok(None) };
    let groups = MapCell::new(HMap::default());
    for x in items {
        let k = it.call1(key, x.clone())?;
        let y = match f {
            Some(f) => it.call1(f, x)?,
            None => x,
        };
        match it.map_find(&groups, &k)? {
            Some(i) => {
                if let Some((_, Value::Array(arr))) = groups.borrow().entries[i].as_ref() {
                    arr.write().push(y);
                }
            }
            None => it.map_set(&groups, k, Value::array(vec![y]))?,
        }
    }
    Ok(Some(groups))
}

/// `groupBy` and `groupMap`: the groups built through `buildC` or `buildCC` in insertion
/// order, as a map through `Map.fromRaw`; a `List` builds its groups as `fromArray` does.
fn group_result(it: &mut Interp, this: &Value, groups: Rc<MapCell>, build: &'static str, companion: Value) -> R<Option<Value>> {
    let list = it.list_shape().filter(|shape| matches!(this, Value::Obj(o) if o.class == shape.cons || Rc::ptr_eq(o, &shape.nil)));
    let entries = Interp::map_entries(&groups);
    let mut pairs = Vec::with_capacity(entries.len());
    for (k, v) in entries {
        let built = match (&list, &v) {
            (Some(shape), Value::Array(arr)) => it.vec_list(shape, arr.borrow().clone()),
            _ => it.call_by_name(this.clone(), build, vec![v])?,
        };
        match it.tuple2(k, built) {
            Some(pair) => pairs.push(pair),
            None => return Ok(Some(it.call_by_name(companion, "fromRaw", vec![Value::Map(groups)])?)),
        }
    }
    store_from(it, pairs, true)
}

fn iterable_group_by(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some(companion) = it.companion_of("Map")? else { return Ok(None) };
    let Some(groups) = grouped(it, this, &arg(a, 0), None)? else { return Ok(None) };
    group_result(it, this, groups, "buildC", companion)
}

fn iterable_group_map(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some(companion) = it.companion_of("Map")? else { return Ok(None) };
    let Some(groups) = grouped(it, this, &arg(a, 0), Some(&arg(a, 1)))? else { return Ok(None) };
    group_result(it, this, groups, "buildCC", companion)
}

fn distinct_by(it: &mut Interp, this: &Value, f: Option<&Value>) -> R<Option<Value>> {
    let Some(items) = it.elements_fast(this)? else { return Ok(None) };
    let seen = MapCell::new(HMap::default());
    let mut out = Vec::new();
    for x in items {
        let k = match f {
            Some(f) => it.call1(f, x.clone())?,
            None => x.clone(),
        };
        if !it.map_has(&seen, &k)? {
            it.map_set(&seen, k, Value::Bool(true))?;
            out.push(x);
        }
    }
    Ok(Some(it.build(this, "buildC", out)?))
}

fn iterable_distinct_by(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    distinct_by(it, this, Some(&arg(a, 0)))
}

fn iterable_build_c(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Value::Array(_) = arg(a, 0) else { return Ok(None) };
    Ok(Some(it.call_by_name(this.clone(), "buildCC", vec![arg(a, 0)])?))
}

// ---- files ----

/// `Path.of`, `getParent`, `getFileName`, `isAbsolute` and `resolve(String)` of
/// std/javalib/nio.scala: the rules its bodies state, `files::path`'s, Windows' on Windows as the
/// builtin `windowsPaths` says there, run natively; the bodies scan a path a character at a time,
/// which a macro walking its sources' parents pays millions of interpreted steps for.
fn path_text(it: &mut Interp, this: &Value) -> Option<Rc<str>> {
    let Value::Obj(o) = this else { return None };
    match it.field_of(o, "text")? {
        Value::Str(text) => Some(text),
        _ => None,
    }
}

fn new_path(it: &mut Interp, text: String) -> Option<Value> {
    let c = it.typer.class_at(&["java", "nio", "file", "Path"])?;
    let slots = it.ctor_plan(c)?;
    (slots.len() == 1).then(|| it.construct_plain(c, &slots, vec![Value::string(text)]))
}

fn path_of(it: &mut Interp, _this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Value::Str(first) = arg(a, 0) else { return Ok(None) };
    Ok(new_path(it, super::files::path::normalized(&first, cfg!(windows))))
}

fn path_parent(it: &mut Interp, this: &Value, _a: &[Value]) -> R<Option<Value>> {
    let Some(text) = path_text(it, this) else { return Ok(None) };
    Ok(match super::files::path::parent(&text, cfg!(windows)) {
        Some(parent) => new_path(it, parent.to_string()),
        None => Some(Value::Null),
    })
}

fn path_file_name(it: &mut Interp, this: &Value, _a: &[Value]) -> R<Option<Value>> {
    let Some(text) = path_text(it, this) else { return Ok(None) };
    Ok(match super::files::path::file_name(&text, cfg!(windows)) {
        Some(name) => new_path(it, name.to_string()),
        None => Some(Value::Null),
    })
}

fn path_is_absolute(it: &mut Interp, this: &Value, _a: &[Value]) -> R<Option<Value>> {
    let Some(text) = path_text(it, this) else { return Ok(None) };
    Ok(Some(Value::Bool(super::files::path::is_absolute(&text, cfg!(windows)))))
}

fn path_resolve(it: &mut Interp, this: &Value, a: &[Value]) -> R<Option<Value>> {
    let (Some(text), Value::Str(other)) = (path_text(it, this), arg(a, 0)) else { return Ok(None) };
    Ok(new_path(it, super::files::path::resolve(&text, &other, cfg!(windows))))
}

/// `ArrayList.toArray()` and `toArray(a)`: a copy of the items.
fn array_list_to_array(it: &mut Interp, this: &Value, _a: &[Value]) -> R<Option<Value>> {
    let Value::Obj(o) = this else { return Ok(None) };
    let Some(items) = it.items_of(o, "ArrayList") else { return Ok(None) };
    Ok(Some(Value::array(items)))
}

/// `Vector.wrap(items)`, and the `buildCC` of `Vector`, `Range` and `IndexedSeq`, which is it.
fn vector_wrap(it: &mut Interp, _this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Some(Value::Array(items)) = a.first() else { return Ok(None) };
    let items = items.borrow().clone();
    it.flat_vector(items)
}

/// `Seq.hashCode`: `seqHash(this)`.
fn seq_hash_code(it: &mut Interp, this: &Value, _a: &[Value]) -> R<Option<Value>> {
    Ok(Some(Value::Int(super::builtins::seq_hash(it, this)?)))
}

// ---- the quoted layer's Scala helpers ----

/// `ToExpr.lit(c)`: `Literal(Constant(c)).asExpr`.
fn to_expr_lit(it: &mut Interp, _this: &Value, a: &[Value]) -> R<Option<Value>> {
    let c = arg(a, 0);
    if !matches!(c, Value::Int(_) | Value::Long(_) | Value::Double(_) | Value::Float(_) | Value::Byte(_) | Value::Short(_) | Value::Bool(_) | Value::Char(_) | Value::Str(_) | Value::Unit | Value::Null) {
        return Ok(None);
    }
    Ok(Some(Value::Tree(TreeRef::Expr(it.literal_expr(&c)?))))
}

/// `FromExpr.constant(x)(pick)`: the literal under the typed wrappers of `x`, given to `pick`.
fn from_expr_constant(it: &mut Interp, _this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Ok(mut t) = it.tree_arg(a, 0) else { return Ok(None) };
    let pick = arg(a, 2);
    for _ in 0..16 {
        match it.view(t)? {
            super::quoted::View::Literal(v) => return Ok(Some(it.call1(&pick, v)?)),
            super::quoted::View::Typed(inner, _) => t = inner,
            _ => return Ok(None),
        }
    }
    Ok(None)
}

/// `Tree.asExpr`: the term as an `Expr`; a method awaiting its arguments is left to the body,
/// which throws.
fn tree_as_expr(it: &mut Interp, _this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Ok(t) = it.tree_arg(a, 0) else { return Ok(None) };
    let Some(term) = it.reflect_class("Term") else { return Ok(None) };
    if !it.tree_is_instance(t, term) || it.awaits_arguments(t)? {
        return Ok(None);
    }
    Ok(Some(Value::Tree(TreeRef::Expr(it.materialize(t)?))))
}

/// `Tree.asExprOf[T]`: the term as an `Expr[T]` when its type conforms.
fn tree_as_expr_of(it: &mut Interp, _this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Ok(t) = it.tree_arg(a, 0) else { return Ok(None) };
    let Ok(target) = it.type_arg(a, 1) else { return Ok(None) };
    let Some(term) = it.reflect_class("Term") else { return Ok(None) };
    if !it.tree_is_instance(t, term) || it.awaits_arguments(t)? {
        return Ok(None);
    }
    let actual = it.tree_type(t)?;
    if !it.conforms(actual, target) {
        return Ok(None);
    }
    Ok(Some(Value::Tree(TreeRef::Expr(it.materialize(t)?))))
}

/// `Expr.asExprOf[X]`: the expression itself when its type conforms.
fn expr_as_expr_of(it: &mut Interp, _this: &Value, a: &[Value]) -> R<Option<Value>> {
    let Ok(t) = it.tree_arg(a, 0) else { return Ok(None) };
    let Ok(target) = it.type_arg(a, 1) else { return Ok(None) };
    let actual = it.tree_type(t)?;
    if !it.conforms(actual, target) {
        return Ok(None);
    }
    Ok(Some(arg(a, 0)))
}
