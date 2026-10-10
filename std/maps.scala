package scala

@jvmClass("java/util/LinkedHashMap")
final class RawMap[K, V]

@js("new $HMap()")
@jvm("new java/util/LinkedHashMap dup invokespecial java/util/LinkedHashMap.<init>()V")
def newRawMap[K, V]: RawMap[K, V]

extension [K, V](raw: RawMap[K, V])
  @js("$0.has($1)")
  @jvm("$0 rt $1:L rtcall wrapKey(Ljava/lang/Object;)Ljava/lang/Object; invokevirtual java/util/LinkedHashMap.containsKey(Ljava/lang/Object;)Z")
  def rawHas(key: K): Boolean
  @js("$0.get($1)")
  @jvm("$0 rt $1:L rtcall wrapKey(Ljava/lang/Object;)Ljava/lang/Object; invokevirtual java/util/LinkedHashMap.get(Ljava/lang/Object;)Ljava/lang/Object;")
  def rawGet(key: K): V
  @js("$0.set($1, $2)")
  @jvm("$0 rt $1:L rtcall wrapKey(Ljava/lang/Object;)Ljava/lang/Object; $2:L invokevirtual java/util/LinkedHashMap.put(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object; pop")
  def rawSet(key: K, value: V): Unit
  @js("$0.delete($1)")
  @jvm("$0 invokevirtual java/util/LinkedHashMap.keySet()Ljava/util/Set; rt $1:L rtcall wrapKey(Ljava/lang/Object;)Ljava/lang/Object; invokeinterface java/util/Set.remove(Ljava/lang/Object;)Z")
  def rawDelete(key: K): Boolean
  @js("$0.keys()")
  @jvm("rt new java/util/ArrayList dup $0 invokevirtual java/util/LinkedHashMap.keySet()Ljava/util/Set; invokespecial java/util/ArrayList.<init>(Ljava/util/Collection;)V rtcall unwrapKeys(Ljava/util/ArrayList;)Ljava/util/ArrayList;")
  def rawKeys: RawBuffer[K]
  @js("$0.values()")
  @jvm("new java/util/ArrayList dup $0 invokevirtual java/util/LinkedHashMap.values()Ljava/util/Collection; invokespecial java/util/ArrayList.<init>(Ljava/util/Collection;)V")
  def rawValues: RawBuffer[V]
  @js("$0.m.size")
  @jvm("invokevirtual java/util/LinkedHashMap.size()I")
  def rawSize: Int
  @js("$0.clear()")
  @jvm("invokevirtual java/util/LinkedHashMap.clear()V")
  def rawClear(): Unit
  @js("$0.forEach($1)")
  def rawForeach[U](f: (K, V) => U): Unit =
    val ks = raw.rawKeys
    var i = 0
    while i < ks.length do
      f(ks(i), raw.rawGet(ks(i)))
      i += 1

// ---- the hash trie index of the immutable Map and Set ----
//
// A version of the immutable Map or Set is an index from a key to the ordinal of its entry in
// a Vector that keeps the insertion order: the index is a CHAMP trie over five bits of the
// key's `##` per level, whose nodes an update copies along one path, so that every version
// shares the rest with the versions before it. The data triples (key, hash, ordinal) stand in
// front of a node's content in the order of their bits, the sub-nodes at its end in the
// reverse order; after the seven levels that consume the 32 bits of a hash, a node holds the
// triples of the keys whose hashes agree entirely and is searched linearly. The hash stays with
// the key so that a rebuild of the index (a compaction) runs no `hashCode` or `equals` of a
// key, which could reach the map being rebuilt. A removed entry leaves a tombstone in the
// Vector, which keeps the order of the rest, and a version whose tombstones outnumber its
// live entries rebuilds both once, memoised on that version so that the versions branching
// from it share the rebuild. Up to `smallStoreSize` entries the index is left out and a lookup
// scans the entries.

// The equality of two keys of the immutable Map and Set. On JavaScript what a native Map gives
// for numbers, strings and the rest (1 and 1L are two keys, NaN is one, 0.0 and -0.0 are one)
// and `equals` between two objects; on the JVM and in the interpreter Scala's `==`, cooperative
// between the boxed numbers.
@js("$keyEq($0, $1)")
def keyEquals(a: Any, b: Any): Boolean = a == b

// The maps are written only by `trieInsertInPlace`, into a node the bulk constructors' builder
// made and no version holds yet.
final class TrieNode(var dataMap: Int, var nodeMap: Int, val content: RawBuffer[Any])

val smallStoreSize: Int = 8

def trieBit(hash: Int, shift: Int): Int = 1 << ((hash >>> shift) & 31)
def trieRank(bitmap: Int, bit: Int): Int = java.lang.Integer.bitCount(bitmap & (bit - 1))
def trieEmpty: TrieNode = new TrieNode(0, 0, emptyBuffer[Any])

// The ordinal of `key`, or -1.
def trieFind(node: TrieNode, key: Any, hash: Int, shift: Int): Int =
  if shift > 30 then trieScan(node.content, key, hash)
  else
    val bit = trieBit(hash, shift)
    if (node.dataMap & bit) != 0 then
      val i = 3 * trieRank(node.dataMap, bit)
      if unsafeCast[Any, Int](node.content(i + 1)) == hash && keyEquals(node.content(i), key) then unsafeCast(node.content(i + 2)) else -1
    else if (node.nodeMap & bit) != 0 then
      trieFind(unsafeCast(node.content(node.content.length - 1 - trieRank(node.nodeMap, bit))), key, hash, shift + 5)
    else -1

def trieScan(triples: RawBuffer[Any], key: Any, hash: Int): Int =
  var i = 0
  var found = -1
  while found < 0 && i < triples.length do
    if unsafeCast[Any, Int](triples(i + 1)) == hash && keyEquals(triples(i), key) then found = unsafeCast(triples(i + 2))
    i += 3
  found

// The node with `key`, which it does not hold, at `ordinal`.
def trieInsert(node: TrieNode, key: Any, hash: Int, shift: Int, ordinal: Int): TrieNode =
  if shift > 30 then new TrieNode(0, 0, contentWithTriple(node.content, node.content.length, key, hash, ordinal))
  else
    val bit = trieBit(hash, shift)
    if (node.dataMap & bit) != 0 then
      val i = 3 * trieRank(node.dataMap, bit)
      val sub = trieMerge(node.content(i), unsafeCast(node.content(i + 1)), unsafeCast(node.content(i + 2)), key, hash, ordinal, shift + 5)
      val at = 3 * (java.lang.Integer.bitCount(node.dataMap) - 1) + java.lang.Integer.bitCount(node.nodeMap) - trieRank(node.nodeMap, bit)
      new TrieNode(node.dataMap ^ bit, node.nodeMap | bit, contentTripleToNode(node.content, i, at, sub))
    else if (node.nodeMap & bit) != 0 then
      val j = node.content.length - 1 - trieRank(node.nodeMap, bit)
      new TrieNode(node.dataMap, node.nodeMap, contentSet(node.content, j, trieInsert(unsafeCast(node.content(j)), key, hash, shift + 5, ordinal)))
    else new TrieNode(node.dataMap | bit, node.nodeMap, contentWithTriple(node.content, 3 * trieRank(node.dataMap, bit), key, hash, ordinal))

// The content arrays an update builds, made at their size and filled by index.
def contentWithTriple(old: RawBuffer[Any], at: Int, key: Any, hash: Int, ordinal: Int): RawBuffer[Any] =
  val out = sizedBuffer[Any](old.length + 3, null)
  var p = 0
  while p < at do
    out(p) = old(p)
    p += 1
  out(at) = key
  out(at + 1) = hash
  out(at + 2) = ordinal
  while p < old.length do
    out(p + 3) = old(p)
    p += 1
  out
def contentWithout(old: RawBuffer[Any], at: Int, count: Int): RawBuffer[Any] =
  val out = sizedBuffer[Any](old.length - count, null)
  var p = 0
  while p < at do
    out(p) = old(p)
    p += 1
  while p + count < old.length do
    out(p) = old(p + count)
    p += 1
  out
def contentSet(old: RawBuffer[Any], at: Int, value: Any): RawBuffer[Any] =
  val out = copyArray(old)
  out(at) = value
  out
// The triple at `i` taken out and `sub` put at `at` of the result.
def contentTripleToNode(old: RawBuffer[Any], i: Int, at: Int, sub: Any): RawBuffer[Any] =
  val out = sizedBuffer[Any](old.length - 2, null)
  var p = 0
  var q = 0
  while p < old.length do
    if p == i then p += 3
    else
      if q == at then q += 1
      out(q) = old(p)
      p += 1
      q += 1
  out(at) = sub
  out
// The node at `j` taken out and the triple put at `i` of the result, `i` before `j`.
def contentNodeToTriple(old: RawBuffer[Any], j: Int, i: Int, key: Any, hash: Int, ordinal: Int): RawBuffer[Any] =
  val out = sizedBuffer[Any](old.length + 2, null)
  var p = 0
  var q = 0
  while p < old.length do
    if p == j then p += 1
    else
      if q == i then q += 3
      out(q) = old(p)
      p += 1
      q += 1
  out(i) = key
  out(i + 1) = hash
  out(i + 2) = ordinal
  out

// The node holding two keys that met in one slot at the level above.
def trieMerge(k0: Any, h0: Int, o0: Int, k1: Any, h1: Int, o1: Int, shift: Int): TrieNode =
  if shift > 30 then
    val out = emptyBuffer[Any]
    out.push(k0)
    out.push(h0)
    out.push(o0)
    out.push(k1)
    out.push(h1)
    out.push(o1)
    new TrieNode(0, 0, out)
  else
    val b0 = trieBit(h0, shift)
    val b1 = trieBit(h1, shift)
    if b0 == b1 then
      val out = emptyBuffer[Any]
      out.push(trieMerge(k0, h0, o0, k1, h1, o1, shift + 5))
      new TrieNode(0, b0, out)
    else
      val out = emptyBuffer[Any]
      if (b0 & (b1 - 1)) != 0 then
        out.push(k0)
        out.push(h0)
        out.push(o0)
        out.push(k1)
        out.push(h1)
        out.push(o1)
      else
        out.push(k1)
        out.push(h1)
        out.push(o1)
        out.push(k0)
        out.push(h0)
        out.push(o0)
      new TrieNode(b0 | b1, 0, out)

// The node without `key`, which it holds.
def trieRemove(node: TrieNode, key: Any, hash: Int, shift: Int): TrieNode =
  if shift > 30 then
    var i = 0
    while i < node.content.length && !(unsafeCast[Any, Int](node.content(i + 1)) == hash && keyEquals(node.content(i), key)) do i += 3
    new TrieNode(0, 0, contentWithout(node.content, i, 3))
  else
    val bit = trieBit(hash, shift)
    if (node.dataMap & bit) != 0 then new TrieNode(node.dataMap ^ bit, node.nodeMap, contentWithout(node.content, 3 * trieRank(node.dataMap, bit), 3))
    else
      val j = node.content.length - 1 - trieRank(node.nodeMap, bit)
      val sub = trieRemove(unsafeCast(node.content(j)), key, hash, shift + 5)
      if sub.content.length == 0 then new TrieNode(node.dataMap, node.nodeMap ^ bit, contentWithout(node.content, j, 1))
      else if sub.nodeMap == 0 && sub.content.length == 3 then
        new TrieNode(node.dataMap | bit, node.nodeMap ^ bit, contentNodeToTriple(node.content, j, 3 * trieRank(node.dataMap, bit), sub.content(0), unsafeCast(sub.content(1)), unsafeCast(sub.content(2))))
      else new TrieNode(node.dataMap, node.nodeMap, contentSet(node.content, j, sub))

// The index of the keys of `entries` with the ordinal of each: the entries themselves, or
// their first parts when they are `pairs`. A transient build: the nodes are this call's own
// until it returns, so every insertion writes into them in place.
def trieOf(entries: RawBuffer[Any], pairs: Boolean): TrieNode =
  val root = trieEmpty
  var i = 0
  while i < entries.length do
    val k = if pairs then unsafeCast[Any, (Any, Any)](entries(i))._1 else entries(i)
    trieInsertInPlace(root, k, k.##, 0, i)
    i += 1
  root

// The index `old` with each entry's ordinal replaced by `ordinals(ordinal)`, the entries whose
// slot holds -1 left out: a compaction, built in place from the stored hashes, so that no
// `hashCode` or `equals` of a key runs.
def trieRebuilt(old: TrieNode, ordinals: RawBuffer[Int]): TrieNode =
  val root = trieEmpty
  trieCopyInto(root, old, ordinals)
  root

def trieCopyInto(root: TrieNode, node: TrieNode, ordinals: RawBuffer[Int]): Unit =
  val triples = if node.dataMap == 0 && node.nodeMap == 0 then node.content.length else 3 * java.lang.Integer.bitCount(node.dataMap)
  var i = 0
  while i < triples do
    val ordinal = ordinals(unsafeCast[Any, Int](node.content(i + 2)))
    if ordinal >= 0 then trieInsertInPlace(root, node.content(i), unsafeCast(node.content(i + 1)), 0, ordinal)
    i += 3
  var j = triples
  while j < node.content.length do
    trieCopyInto(root, unsafeCast(node.content(j)), ordinals)
    j += 1

// `trieInsert` into a node no version holds: the content array and the maps change in place.
def trieInsertInPlace(node: TrieNode, key: Any, hash: Int, shift: Int, ordinal: Int): Unit =
  if shift > 30 then
    node.content.push(key)
    node.content.push(hash)
    node.content.push(ordinal)
  else
    val bit = trieBit(hash, shift)
    if (node.dataMap & bit) != 0 then
      val i = 3 * trieRank(node.dataMap, bit)
      val sub = trieMerge(node.content(i), unsafeCast(node.content(i + 1)), unsafeCast(node.content(i + 2)), key, hash, ordinal, shift + 5)
      scala.collection.mutable.bufferRemoveRange(node.content, i, 3)
      scala.collection.mutable.bufferInsert(node.content, 3 * (java.lang.Integer.bitCount(node.dataMap) - 1) + java.lang.Integer.bitCount(node.nodeMap) - trieRank(node.nodeMap, bit), sub)
      node.dataMap = node.dataMap ^ bit
      node.nodeMap = node.nodeMap | bit
    else if (node.nodeMap & bit) != 0 then
      trieInsertInPlace(unsafeCast(node.content(node.content.length - 1 - trieRank(node.nodeMap, bit))), key, hash, shift + 5, ordinal)
    else
      val i = 3 * trieRank(node.dataMap, bit)
      scala.collection.mutable.bufferInsert(node.content, i, ordinal)
      scala.collection.mutable.bufferInsert(node.content, i, hash)
      scala.collection.mutable.bufferInsert(node.content, i, key)
      node.dataMap = node.dataMap | bit

// Two RawMaps compared and hashed as maps: the mutable HashMap's equality and hash.
@js("$mapEquals($0, $1)")
@jvm("rt $0:L $1:L rtcall mapEquals(Ljava/lang/Object;Ljava/lang/Object;)Z")
def rawMapEquals(a: Any, b: Any): Boolean

@js("$mapHash($0)")
@jvm("rt $0:L rtcall mapHash(Ljava/lang/Object;)I")
def rawMapHash(a: Any): Int

// A map is `get`, `iterator`, `updated` and `removed`; the rest follows from them. A class of a
// library implements the four, the std's own map is `PersistentMap` over the hash trie index.
@predef
abstract class Map[K, +V] extends IterableOps[(K, V), Iterable, Map[K, V]], scala.collection.Map[K, V]:
  def get(key: K): Option[V]
  def iterator: Iterator[(K, V)]
  def updated[V1 >: V](key: K, value: V1): Map[K, V1]
  def removed(key: K): Map[K, V]
  def buildCC[B](items: RawBuffer[B]): Iterable[B] = fromArray(items)
  override def buildC(items: RawBuffer[(K, V)]): Map[K, V] = Map.from(Vector.wrap(items))
  override def view: MapView[K, V] = new MapView(() => iterator, k => get(k))
  def foreach[U](f: ((K, V)) => U): Unit = iterator.foreach(f)
  override def contains(key: K): Boolean = get(key).isDefined
  override def isDefinedAt(key: K): Boolean = contains(key)
  override def apply(key: K): V = get(key) match
    case Some(v) => v
    case None => default(key)
  def default(key: K): V = noSuchElement("key not found: " + key)
  override def getOrElse[V1 >: V](key: K, default: => V1): V1 = get(key) match
    case Some(v) => v
    case None => default
  def applyOrElse[V1 >: V](key: K, default: K => V1): V1 = get(key) match
    case Some(v) => v
    case None => default(key)
  def withDefaultValue[V1 >: V](value: V1): Map[K, V1] = PersistentMap.of(this).withDefaultValue(value)
  def withDefault[V1 >: V](f: K => V1): Map[K, V1] = PersistentMap.of(this).withDefault(f)
  def +[V1 >: V](entry: (K, V1)): Map[K, V1] = updated(entry._1, entry._2)
  def +[V1 >: V](entry1: (K, V1), entry2: (K, V1), entries: (K, V1)*): Map[K, V1] = this + entry1 + entry2 ++ entries
  def updatedWith[V1 >: V](key: K)(remap: Option[V] => Option[V1]): Map[K, V1] = remap(get(key)) match
    case Some(v) => updated(key, v)
    case None => removed(key)
  def -(key: K): Map[K, V] = removed(key)
  def removedAll(keys: IterableOnce[K]): Map[K, V] =
    var out: Map[K, V] = this
    keys.foreach(k => out = out.removed(k))
    out
  def --(keys: IterableOnce[K]): Map[K, V] = removedAll(keys)
  def ++[V1 >: V](that: IterableOnce[(K, V1)]): Map[K, V1] =
    var out: Map[K, V1] = this
    that.foreach(e => out = out.updated(e._1, e._2))
    out
  override def concat[V1 >: V](that: IterableOnce[(K, V1)]): Map[K, V1] = this ++ that

  def keys: List[K] = fromArray(rawItems(this).map(e => e._1))
  def values: List[V] = fromArray(rawItems(this).map(e => e._2))
  def keySet: Set[K] = Set.from(keys)
  def keysIterator: Iterator[K] = iterator.map(e => e._1)
  def valuesIterator: Iterator[V] = iterator.map(e => e._2)
  override def toMap[K2, V2](implicit ev: ((K, V)) <:< (K2, V2)): Map[K2, V2] = unsafeCast(this)
  // A function over the entries that gives pairs builds a Map, any other an Iterable, as
  // scala-library's overloads of `MapOps` and `IterableOps` do.
  @jvmWide
  def map[K2, V2](f: ((K, V)) => (K2, V2)): Map[K2, V2] = Map.fromResults(rawItems(this).map(f))
  override def map[B](f: ((K, V)) => B): Iterable[B] = fromArray(rawItems(this).map(f))
  @jvmWide
  def collect[K2, V2](pf: PartialFunction[(K, V), (K2, V2)]): Map[K2, V2] =
    Map.fromResults(Vector.wrap(rawItems(this)).collect(pf).unsafeArray)
  override def collect[B](pf: PartialFunction[(K, V), B]): Iterable[B] = fromArray(Vector.wrap(rawItems(this)).collect(pf).unsafeArray)
  @jvmWide
  def flatMap[K2, V2](f: ((K, V)) => IterableOnce[(K2, V2)]): Map[K2, V2] =
    Map.fromResults(Vector.wrap(rawItems(this)).flatMap(f).unsafeArray)
  override def flatMap[B](f: ((K, V)) => IterableOnce[B]): Iterable[B] = fromArray(Vector.wrap(rawItems(this)).flatMap(f).unsafeArray)
  def filterKeys(p: K => Boolean): Map[K, V] = filter(e => p(e._1))
  override def mkString(start: String, sep: String, end: String): String =
    joinStrings(Vector.wrap(rawItems(this).map(e => "" + e._1 + " -> " + e._2)), start, sep, end)

  def equals(that: Any): Boolean = that match
    case m: Map[?, ?] =>
      val other: Map[K, Any] = unsafeCast(m)
      size == other.size && forall(e => other.get(e._1) match
        case Some(v) => v == e._2
        case None => false)
    case _ => false
  override def hashCode: Int = scala.util.hashing.MurmurHash3.mapHash(this)
  override def collectionClassName: String = if size <= 4 then "Map" else "HashMap"
  override def toString: String = mkString("Map(", ", ", ")")

// The std's map: the trie index from a key to the ordinal of its entry, the entries in
// insertion order with a tombstone where one was removed, and the count of the tombstones. The
// empty map is one shared instance, as in Scala; adding to it starts a fresh entry array, so
// that no version appends in place to the array the shared instance holds. `fallback` answers
// `apply` for a missing key after withDefault or withDefaultValue.
final class PersistentMap[K, +V](index: TrieNode, entries: Vector[Any], dead: Int, fallback: Option[K => Any], private var compacted: PersistentMap[K, Any] = null) extends Map[K, V]:
  override def size: Int = entries.length - dead
  override def knownSize: Int = size
  override def isEmpty: Boolean = size == 0
  private def ordinalOf(key: K): Int = if index == null then scanOrdinal(key) else trieFind(index, key, key.##, 0)
  private def scanOrdinal(key: K): Int =
    var i = 0
    var found = -1
    while found < 0 && i < entries.length do
      if keyEquals(unsafeCast[Any, (K, V)](entries(i))._1, key) then found = i
      i += 1
    found
  private def entryAt(ordinal: Int): (K, V) = unsafeCast(entries(ordinal))
  override def contains(key: K): Boolean = ordinalOf(key) >= 0
  override def isDefinedAt(key: K): Boolean = ordinalOf(key) >= 0
  def get(key: K): Option[V] =
    val i = ordinalOf(key)
    if i >= 0 then Some(entryAt(i)._2) else None
  override def apply(key: K): V =
    val i = ordinalOf(key)
    if i >= 0 then entryAt(i)._2
    else fallback match
      case Some(f) => unsafeCast(f(key))
      case None => default(key)
  override def getOrElse[V1 >: V](key: K, default: => V1): V1 =
    val i = ordinalOf(key)
    if i >= 0 then entryAt(i)._2 else default
  override def applyOrElse[V1 >: V](key: K, default: K => V1): V1 =
    val i = ordinalOf(key)
    if i >= 0 then entryAt(i)._2 else default(key)
  override def withDefaultValue[V1 >: V](value: V1): Map[K, V1] = new PersistentMap(index, entries, dead, Some(_ => value))
  override def withDefault[V1 >: V](f: K => V1): Map[K, V1] = new PersistentMap(index, entries, dead, Some(f))
  // The entries a traversal kept, which are distinct, as a map with the same fallback.
  override def buildC(items: RawBuffer[(K, V)]): Map[K, V] =
    if items.length == 0 && fallback.isEmpty then Map.empty
    else new PersistentMap(if items.length <= smallStoreSize then null else trieOf(unsafeCast(items), true), Vector.wrap(unsafeCast(items)), 0, fallback)

  def updated[V1 >: V](key: K, value: V1): Map[K, V1] =
    if index == null then
      val i = scanOrdinal(key)
      if i >= 0 then new PersistentMap(null, entries.updated(i, (entryAt(i)._1, value)), 0, fallback)
      else if entries.length == 0 then new PersistentMap(null, Vector.wrap(arrayOfOne((key, value))), 0, fallback)
      else if entries.length < smallStoreSize then new PersistentMap(null, entries :+ (key, value), 0, fallback)
      else
        val next = entries :+ (key, value)
        new PersistentMap(trieOf(next.unsafeArray, true), next, 0, fallback)
    else
      val hash = key.##
      val i = trieFind(index, key, hash, 0)
      if i >= 0 then new PersistentMap(index, entries.updated(i, (entryAt(i)._1, value)), dead, fallback)
      else new PersistentMap(trieInsert(index, key, hash, 0, entries.length), entries :+ (key, value), dead, fallback)
  def removed(key: K): Map[K, V] =
    if index == null then
      val i = scanOrdinal(key)
      if i < 0 then this
      else if size == 1 && fallback.isEmpty then Map.empty
      else new PersistentMap(null, Vector.wrap(PersistentMap.without(entries, i)), 0, fallback)
    else removedHashed(key, key.##)
  // A compacted map may be small enough to have no index, so the scan stays a possibility.
  private def removedHashed(key: K, hash: Int): Map[K, V] =
    val i = if index == null then scanOrdinal(key) else trieFind(index, key, hash, 0)
    if i < 0 then this
    else if size == 1 && fallback.isEmpty then Map.empty
    else if index == null then new PersistentMap(null, Vector.wrap(PersistentMap.without(entries, i)), 0, fallback)
    else if dead + 1 > 16 && 2 * (dead + 1) > entries.length then unsafeCast(compact.removedHashed(key, hash))
    else new PersistentMap(trieRemove(index, key, hash, 0), PersistentMap.buried(entries, i), dead + 1, fallback)
  // The same map without its tombstones, built once for every version that branches from this
  // one, from the stored hashes: no key's `hashCode` or `equals` runs.
  private def compact: PersistentMap[K, Any] =
    if compacted == null then
      val live = PersistentMap.without(entries, -1)
      compacted = new PersistentMap(if live.length <= smallStoreSize then null else trieRebuilt(index, PersistentMap.compactedOrdinals(entries)), Vector.wrap(live), 0, fallback)
    compacted

  override def foreach[U](f: ((K, V)) => U): Unit =
    if dead == 0 then entries.foreach(e => f(unsafeCast(e)))
    else entries.foreach(e => if PersistentMap.live(e) then f(unsafeCast(e)))
  def iterator: Iterator[(K, V)] =
    if dead == 0 then unsafeCast(entries.iterator) else unsafeCast(entries.iterator.filter(PersistentMap.live))
  @jvmEvidence
  @jvm("rt $0 $1 rtcall arrayOf(Lscala/IterableOnce;Lscala/reflect/ClassTag;)Ljava/lang/Object; cast_result")
  override def toArray[B >: (K, V): scala.reflect.ClassTag]: Array[B] = if dead == 0 then unsafeCast(entries.toArray) else unsafeCast(PersistentMap.without(entries, -1))
  private def parts[B](first: Boolean): RawBuffer[B] =
    val out = emptyBuffer[B]
    foreach(e => out.push(unsafeCast(if first then e._1 else e._2)))
    out
  override def keys: List[K] = fromArray(parts(true))
  override def values: List[V] = fromArray(parts(false))
  override def keySet: Set[K] = scala.collection.immutable.HashSet.fromDistinct(parts[K](true))
  override def keysIterator: Iterator[K] = Iterator.over(parts(true))
  override def valuesIterator: Iterator[V] = Iterator.over(parts(false))
  // A bulk addition at least as large as the map rebuilds the index in place; a smaller one
  // copies a path per entry.
  override def ++[V1 >: V](that: IterableOnce[(K, V1)]): Map[K, V1] =
    val added = iterableToArray(that)
    if added.length == 0 then this
    else if index == null || added.length >= size then
      val all: RawBuffer[(K, V1)] = unsafeCast(rawItems(this))
      added.foreach(e => all.push(e))
      PersistentMap.of(Map.from(Vector.wrap(all))).withFallback(fallback)
    else
      var out: Map[K, V1] = this
      added.foreach(e => out = out.updated(e._1, e._2))
      out
  private def withFallback[V1](f: Option[K => Any]): Map[K, V1] = if f.isEmpty then unsafeCast(this) else new PersistentMap(index, entries, dead, f)

object PersistentMap:
  // The slot of a removed entry: an instance of a class of this object's own, which nothing
  // outside it can name, so no element can pass for one.
  private final class Tombstone
  private val tombstone: Any = new Tombstone
  def live(e: Any): Boolean = !(e eq tombstone)
  // The entries with the one at `i` removed.
  def buried(entries: Vector[Any], i: Int): Vector[Any] = entries.updated(i, tombstone)
  // The entries but the one at `dropped` and the tombstones.
  def without(entries: Vector[Any], dropped: Int): RawBuffer[Any] =
    val out = emptyBuffer[Any]
    var i = 0
    while i < entries.length do
      val e = entries(i)
      if i != dropped && live(e) then out.push(e)
      i += 1
    out
  // The new ordinal of each entry once the tombstones are dropped, -1 for a tombstone.
  def compactedOrdinals(entries: Vector[Any]): RawBuffer[Int] =
    val out = sizedBuffer[Int](entries.length, -1)
    var next = 0
    var i = 0
    while i < entries.length do
      if live(entries(i)) then
        out(i) = next
        next += 1
      i += 1
    out
  def of[K, V](m: Map[K, V]): PersistentMap[K, V] = m match
    case p: PersistentMap[?, ?] => unsafeCast(p)
    case _ => unsafeCast(Map.from(m.toArray))
  // A map over entries whose keys are distinct.
  def fromEntries[K, V](entries: RawBuffer[(K, V)]): Map[K, V] =
    if entries.length == 0 then Map.empty
    else new PersistentMap(if entries.length <= smallStoreSize then null else trieOf(unsafeCast(entries), true), Vector.wrap(unsafeCast(entries)), 0, None)

extension [K, V](m: Map[K, V])
  def transform[W](f: (K, V) => W): Map[K, W] = m.map(e => (e._1, f(e._1, e._2)))
  def mapValues[W](f: V => W): Map[K, W] = m.map(e => (e._1, f(e._2)))

// A map traversed and looked up on demand: mapValues and filterKeys apply their function when
// an entry is asked for, and toMap materialises the result.
final class MapView[K, +V](entries: () => Iterator[(K, V)], lookup: K => Option[V]) extends IterableOps[(K, V), View, MapView[K, V]], Iterable[(K, V)]:
  def buildCC[B](items: RawBuffer[B]): View[B] = new View(() => Iterator.over(items), "View")
  override def buildC(items: RawBuffer[(K, V)]): MapView[K, V] = Map.from(Vector.wrap(items)).view
  override def iterator: Iterator[(K, V)] = entries()
  def foreach[U](f: ((K, V)) => U): Unit = entries().foreach(f)
  override def view: MapView[K, V] = this
  def get(key: K): Option[V] = lookup(key)
  def apply(key: K): V = lookup(key) match
    case Some(v) => v
    case None => noSuchElement("key not found: " + key)
  def getOrElse[V1 >: V](key: K, default: => V1): V1 = lookup(key).getOrElse(default)
  def contains(key: K): Boolean = lookup(key).isDefined
  def keys: View[K] = new View(() => entries().map(e => e._1), "View")
  def values: View[V] = new View(() => entries().map(e => e._2), "View")
  def keySet: Set[K] = Set.from(keys)
  def mapValues[W](f: V => W): MapView[K, W] = new MapView(() => entries().map(e => (e._1, f(e._2))), k => lookup(k).map(f))
  def map[K2, V2](f: ((K, V)) => (K2, V2)): View[(K2, V2)] = new View(() => entries().map(f), "View")
  override def map[B](f: ((K, V)) => B): View[B] = new View(() => entries().map(f), "View")
  def ++[V2 >: V](that: IterableOnce[(K, V2)]): View[(K, V2)] = new View(() => entries() ++ that.iterator, "View")
  def filterKeys(p: K => Boolean): MapView[K, V] = new MapView(() => entries().filter(e => p(e._1)), k => if p(k) then lookup(k) else None)
  override def filter(p: ((K, V)) => Boolean): MapView[K, V] =
    new MapView(() => entries().filter(p), k => lookup(k).filter(v => p((k, v))))
  override def toMap[K2, V2](implicit ev: ((K, V)) <:< (K2, V2)): Map[K2, V2] = unsafeCast(Map.from(this))
  override def toString: String = "MapView(<not computed>)"

/** The companion of a map: scala-library's `scala.collection.MapFactory`. */
trait MapFactory[+CC[_, _]]:
  def empty[K, V]: CC[K, V]
  def from[K, V](it: IterableOnce[(K, V)]): CC[K, V]
  def apply[K, V](elems: (K, V)*): CC[K, V] = from(elems)
  def newBuilder[K, V]: scala.collection.mutable.Builder[(K, V), CC[K, V]]
  implicit def mapFactory[K, V]: Factory[(K, V), CC[K, V]] = MapFactory.toFactory(this)

object MapFactory:
  implicit def toFactory[K, V, CC[_, _]](factory: MapFactory[CC]): Factory[(K, V), CC[K, V]] =
    new Factory[(K, V), CC[K, V]]:
      def fromSpecific(it: IterableOnce[(K, V)]): CC[K, V] = factory.from(it)
      def newBuilder: scala.collection.mutable.Builder[(K, V), CC[K, V]] = factory.newBuilder[K, V]

@predef
object Map extends MapFactory[Map]:
  private val emptyMap: Map[Nothing, Nothing] = new PersistentMap(null, Vector.empty, 0, None)
  def empty[K, V]: Map[K, V] = unsafeCast(emptyMap)
  def fromRaw[K, V](entries: RawMap[K, Any]): Map[K, V] =
    val ks = entries.rawKeys
    val vs = entries.rawValues
    val pairs = emptyBuffer[(K, V)]
    var i = 0
    while i < ks.length do
      pairs.push((ks(i), unsafeCast(vs(i))))
      i += 1
    PersistentMap.fromEntries(pairs)
  def apply[K, V](entries: (K, V)*): Map[K, V] = from(entries)
  def newBuilder[K, V]: scala.collection.mutable.Builder[(K, V), Map[K, V]] =
    scala.collection.mutable.ArrayBuffer.empty[(K, V)].mapResult(from)
  def from[K, V](entries: IterableOnce[(K, V)]): Map[K, V] = entries match
    case m: Map[?, ?] => unsafeCast(m)
    case _ =>
      val raw = newRawMap[K, Any]
      entries.foreach(e => raw.rawSet(e._1, e._2))
      fromRaw(raw)
  // Reached through Iterable, map and flatMap may be handed a function that does not return
  // pairs; the result is a List then, as the static type promises.
  @jvmWide
  def fromResults[K, V](items: RawBuffer[(K, V)]): Map[K, V] =
    if items.forall(isPair) then from(Vector.wrap(items)) else unsafeCast(fromArray(items))
  def isPair(x: Any): Boolean = x match
    case _: Tuple2[?, ?] => true
    case _ => false
  // scala-library's small maps as classes a library body constructs (zio's
  // `new Map.Map1(key, entry)`), over the std's persistent map.
  final class Map1[K, +V](key1: K, value1: V) extends SmallMap[K, V](Map.from(List(key1 -> value1)))
  final class Map2[K, +V](key1: K, value1: V, key2: K, value2: V) extends SmallMap[K, V](Map.from(List(key1 -> value1, key2 -> value2)))
  final class Map3[K, +V](key1: K, value1: V, key2: K, value2: V, key3: K, value3: V)
      extends SmallMap[K, V](Map.from(List(key1 -> value1, key2 -> value2, key3 -> value3)))
  final class Map4[K, +V](key1: K, value1: V, key2: K, value2: V, key3: K, value3: V, key4: K, value4: V)
      extends SmallMap[K, V](Map.from(List(key1 -> value1, key2 -> value2, key3 -> value3, key4 -> value4)))
  abstract class SmallMap[K, +V](entries: Map[K, V]) extends Map[K, V]:
    override def size: Int = entries.size
    override def knownSize: Int = entries.size
    def get(key: K): Option[V] = entries.get(key)
    def iterator: Iterator[(K, V)] = entries.iterator
    def updated[V1 >: V](key: K, value: V1): Map[K, V1] = entries.updated(key, value)
    def removed(key: K): Map[K, V] = entries.removed(key)

// What sets share, with `C` as the type of the set that the operations give back.
trait SetOps[A, +C] extends IterableOnce[A]:
  def contains(elem: A): Boolean
  def incl(elem: A): C
  def excl(elem: A): C
  def concat(that: IterableOnce[A]): C
  def removedAll(that: IterableOnce[A]): C
  def filter(p: A => Boolean): C
  def forall(p: A => Boolean): Boolean
  def apply(elem: A): Boolean = contains(elem)
  def +(elem: A): C = incl(elem)
  def +(elem1: A, elem2: A, elems: A*): C = concat(elems.prepended(elem2).prepended(elem1))
  def -(elem: A): C = excl(elem)
  def ++(that: IterableOnce[A]): C = concat(that)
  def --(that: IterableOnce[A]): C = removedAll(that)
  def union(that: Set[A]): C = concat(that)
  def |(that: Set[A]): C = concat(that)
  def intersect(that: scala.collection.Set[A]): C = filter(x => that.contains(x))
  def &(that: scala.collection.Set[A]): C = filter(x => that.contains(x))
  def diff(that: scala.collection.Set[A]): C = filter(x => !that.contains(x))
  def &~(that: scala.collection.Set[A]): C = filter(x => !that.contains(x))
  def subsetOf(that: scala.collection.Set[A]): Boolean = forall(x => that.contains(x))

@predef
trait Set[A] extends SetOps[A, Set[A]], IterableOps[A, Set, Set[A]], Iterable[A], scala.collection.Set[A]:
  def buildCC[B](items: RawBuffer[B]): Set[B] = Set.from(Vector.wrap(items))
  override def toSet[B >: A]: Set[B] = unsafeCast(this)
  def equals(that: Any): Boolean = that match
    case s: Set[?] =>
      val other: Set[A] = unsafeCast(s)
      size == other.size && forall(x => other.contains(x))
    case _ => false
  override def hashCode: Int = setHash(this)

@predef
object Set:
  // scala-library's companion is an `IterableFactory`, whose `iterableFactory` a library body
  // names as the implicit `Factory` it resolved.
  implicit def iterableFactory[A]: Factory[A, Set[A]] =
    new Factory[A, Set[A]]:
      def fromSpecific(it: IterableOnce[A]): Set[A] = from(it)
      def newBuilder: scala.collection.mutable.Builder[A, Set[A]] = Set.newBuilder[A]
  private val emptySet: Set[Nothing] = new scala.collection.immutable.HashSet(null, Vector.empty, 0)
  def newBuilder[A]: scala.collection.mutable.Builder[A, Set[A]] = scala.collection.mutable.ArrayBuffer.empty[A].mapResult(_.toSet)
  def empty[A]: Set[A] = unsafeCast(emptySet)
  def apply[A](elems: A*): Set[A] = from(elems)
  def from[A](elems: IterableOnce[A]): Set[A] = elems match
    case s: scala.collection.immutable.HashSet[?] => unsafeCast(s)
    case _ =>
      val raw = newRawMap[A, Any]
      elems.foreach(x => raw.rawSet(x, true))
      scala.collection.immutable.HashSet.fromDistinct(raw.rawKeys)
