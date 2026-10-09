package scala.collection.immutable

// The std's set: the trie index from an element to the ordinal of its entry, the elements in
// insertion order with a tombstone where one was removed, and the count of the tombstones, as
// `PersistentMap` keeps its entries. The empty set is one shared instance, as in Scala, and
// adding to it starts a fresh entry array for the same reason.
final class HashSet[A](index: TrieNode, entries: Vector[Any], dead: Int, private var compacted: HashSet[A] = null) extends Set[A]:
  override def size: Int = entries.length - dead
  override def knownSize: Int = size
  override def isEmpty: Boolean = size == 0
  private def ordinalOf(elem: A): Int = if index == null then scanOrdinal(elem) else trieFind(index, elem, elem.##, 0)
  private def scanOrdinal(elem: A): Int =
    var i = 0
    var found = -1
    while found < 0 && i < entries.length do
      if keyEquals(entries(i), elem) then found = i
      i += 1
    found
  def contains(elem: A): Boolean = ordinalOf(elem) >= 0
  def incl(elem: A): Set[A] =
    if index == null then
      if scanOrdinal(elem) >= 0 then this
      else if entries.length == 0 then new HashSet(null, Vector.wrap(arrayOfOne(elem)), 0)
      else if entries.length < smallStoreSize then new HashSet(null, entries :+ elem, 0)
      else
        val next = entries :+ elem
        new HashSet(trieOf(next.unsafeArray, false), next, 0)
    else
      val hash = elem.##
      if trieFind(index, elem, hash, 0) >= 0 then this
      else new HashSet(trieInsert(index, elem, hash, 0, entries.length), entries :+ elem, dead)
  def excl(elem: A): Set[A] =
    if index == null then
      val i = scanOrdinal(elem)
      if i < 0 then this
      else if size == 1 then Set.empty
      else new HashSet(null, Vector.wrap(PersistentMap.without(entries, i)), 0)
    else exclHashed(elem, elem.##)
  private def exclHashed(elem: A, hash: Int): Set[A] =
    val i = if index == null then scanOrdinal(elem) else trieFind(index, elem, hash, 0)
    if i < 0 then this
    else if size == 1 then Set.empty
    else if index == null then new HashSet(null, Vector.wrap(PersistentMap.without(entries, i)), 0)
    else if dead + 1 > 16 && 2 * (dead + 1) > entries.length then compact.exclHashed(elem, hash)
    else new HashSet(trieRemove(index, elem, hash, 0), PersistentMap.buried(entries, i), dead + 1)
  // The same set without its tombstones, built once for every version that branches from this
  // one, from the stored hashes: no element's `hashCode` or `equals` runs.
  private def compact: HashSet[A] =
    if compacted == null then
      val live = PersistentMap.without(entries, -1)
      compacted = new HashSet(if live.length <= smallStoreSize then null else trieRebuilt(index, PersistentMap.compactedOrdinals(entries)), Vector.wrap(live), 0)
    compacted
  // A bulk addition at least as large as the set rebuilds the index in place; a smaller one
  // copies a path per element.
  override def concat(that: IterableOnce[A]): Set[A] =
    val added = iterableToArray(that)
    if added.length == 0 then this
    else if index == null || added.length >= size then
      val all = rawItems(this)
      added.foreach(x => all.push(x))
      Set.from(Vector.wrap(all))
    else
      var out: Set[A] = this
      added.foreach(x => out = out.incl(x))
      out
  def removedAll(that: IterableOnce[A]): Set[A] =
    var out: Set[A] = this
    that.foreach(x => out = out.excl(x))
    out
  def foreach[U](f: A => U): Unit =
    if dead == 0 then entries.foreach(e => f(unsafeCast(e)))
    else entries.foreach(e => if PersistentMap.live(e) then f(unsafeCast(e)))
  override def iterator: Iterator[A] =
    if dead == 0 then unsafeCast(entries.iterator) else unsafeCast(entries.iterator.filter(PersistentMap.live))
  @jvmEvidence
  @jvm("rt $0 $1 rtcall arrayOf(Lscala/IterableOnce;Lscala/reflect/ClassTag;)Ljava/lang/Object; cast_result")
  override def toArray[B >: A: scala.reflect.ClassTag]: Array[B] = if dead == 0 then unsafeCast(entries.toArray) else unsafeCast(PersistentMap.without(entries, -1))
  // A `HashSet` maps to a `HashSet`, as scala-library's does.
  override def map[B](f: A => B): HashSet[B] = HashSet.from(iterator.map(f))
  override def flatMap[B](f: A => IterableOnce[B]): HashSet[B] = HashSet.from(iterator.flatMap(f))
  override def collectionClassName: String = if size <= 4 then "Set" else "HashSet"
  override def toString: String = mkString("Set(", ", ", ")")

object HashSet:
  def empty[A]: HashSet[A] = unsafeCast(Set.empty[A])
  def apply[A](elems: A*): HashSet[A] = from(elems)
  def from[A](elems: IterableOnce[A]): HashSet[A] = unsafeCast(Set.from(elems))
  // A set over elements that are distinct.
  def fromDistinct[A](elems: RawBuffer[A]): Set[A] =
    if elems.length == 0 then Set.empty
    else new HashSet(if elems.length <= smallStoreSize then null else trieOf(unsafeCast(elems), false), Vector.wrap(unsafeCast(elems)), 0)
  def newBuilder[A]: scala.collection.mutable.Builder[A, HashSet[A]] = scala.collection.mutable.ArrayBuffer.empty[A].mapResult(b => from(b))
  // scala-library's companion is an `IterableFactory`, whose `iterableFactory` a library body
  // names as the implicit `Factory` it resolved.
  implicit def iterableFactory[A]: Factory[A, HashSet[A]] =
    new Factory[A, HashSet[A]]:
      def fromSpecific(it: IterableOnce[A]): HashSet[A] = from(it)
      def newBuilder: scala.collection.mutable.Builder[A, HashSet[A]] = HashSet.newBuilder[A]

// A sorted array: lookups are binary searches, updates copy the array like the other sets do.
class SortedSet[A](private val items: RawBuffer[A], val ordering: Ordering[A]) extends SetOps[A, SortedSet[A]], IterableOps[A, Set, SortedSet[A]], Set[A]:
  override def buildC(sorted: RawBuffer[A]): SortedSet[A] = new TreeSet(sorted, ordering)
  override def iterator: Iterator[A] = Iterator.over(items)
  @jvmEvidence
  @jvm("rt $0 $1 rtcall arrayOf(Lscala/IterableOnce;Lscala/reflect/ClassTag;)Ljava/lang/Object; cast_result")
  override def toArray[B >: A: scala.reflect.ClassTag]: Array[B] = unsafeCast(copyArray(items))
  override def size: Int = items.length
  override def knownSize: Int = items.length
  override def isEmpty: Boolean = items.length == 0
  def foreach[U](f: A => U): Unit = items.foreach(f)
  def firstKey: A = head
  def lastKey: A = last
  // The index of elem, or -(insertion point) - 1 without one.
  private def search(elem: A): Int =
    var lo = 0
    var hi = items.length - 1
    var found = -1
    while found < 0 && lo <= hi do
      val mid = (lo + hi) / 2
      val c = ordering.compare(items(mid), elem)
      if c == 0 then found = mid
      else if c < 0 then lo = mid + 1
      else hi = mid - 1
    if found >= 0 then found else -lo - 1
  def contains(elem: A): Boolean = search(elem) >= 0
  def incl(elem: A): SortedSet[A] =
    val i = search(elem)
    if i >= 0 then this
    else
      val copy = copyArray(items)
      scala.collection.mutable.bufferInsert(copy, -i - 1, elem)
      new TreeSet(copy, ordering)
  def excl(elem: A): SortedSet[A] =
    val i = search(elem)
    if i < 0 then this
    else
      val copy = copyArray(items)
      scala.collection.mutable.bufferRemove(copy, i)
      new TreeSet(copy, ordering)
  override def ++(that: IterableOnce[A]): SortedSet[A] = concat(that)
  override def concat(that: IterableOnce[A]): SortedSet[A] =
    val all = copyArray(items)
    that.foreach(x => all.push(x))
    SortedSet.from(Vector.wrap(all))(using ordering)
  def removedAll(that: IterableOnce[A]): SortedSet[A] =
    val dropped = SortedSet.from(that)(using ordering)
    filter(x => !dropped.contains(x))
  def rangeFrom(from: A): SortedSet[A] = filter(x => ordering.gteq(x, from))
  def rangeUntil(until: A): SortedSet[A] = filter(x => ordering.lt(x, until))
  def range(from: A, until: A): SortedSet[A] = filter(x => ordering.gteq(x, from) && ordering.lt(x, until))
  // Called through the static type Set, no ordering is passed and the result is a plain Set.
  override def map[B](f: A => B)(implicit ord: Ordering[B]): SortedSet[B] =
    val out = emptyBuffer[B]
    foreach(x => out.push(f(x)))
    SortedSet.sortedOrPlain(out, ord)
  override def flatMap[B](f: A => IterableOnce[B])(implicit ord: Ordering[B]): SortedSet[B] =
    val out = emptyBuffer[B]
    foreach(x => f(x).iterator.foreach(y => out.push(y)))
    SortedSet.sortedOrPlain(out, ord)
  override def collectionClassName: String = "TreeSet"
  override def toString: String = mkString("TreeSet(", ", ", ")")

object SortedSet extends scala.collection.EvidenceIterableFactory[SortedSet, Ordering]:
  def sortedOrPlain[B](items: RawBuffer[B], ord: Ordering[B]): SortedSet[B] =
    if js.isUndefined(ord) then unsafeCast(Set.from(Vector.wrap(items))) else from(Vector.wrap(items))(using ord)
  def empty[A](implicit ord: Ordering[A]): SortedSet[A] = new TreeSet(emptyBuffer[A], ord)
  def apply[A](elems: A*)(implicit ord: Ordering[A]): SortedSet[A] = from(elems)
  def from[A](elems: IterableOnce[A])(implicit ord: Ordering[A]): TreeSet[A] =
    val sorted = iterableToArray(elems)
    sortArray(sorted, (a, b) => ord.compare(a, b))
    val out = emptyBuffer[A]
    sorted.foreach(x => if out.length == 0 || ord.compare(out(out.length - 1), x) != 0 then out.push(x))
    new TreeSet(out, ord)
  def newBuilder[A](implicit ord: Ordering[A]): scala.collection.mutable.Builder[A, SortedSet[A]] =
    scala.collection.mutable.ArrayBuffer.empty[A].mapResult(items => from(items))

// Every sorted set is a TreeSet, as in scala-library; the class keeps the two types distinct
// for a library's givens per set class (zio-json's `sortedSet` and `treeSet`).
final class TreeSet[A](items: RawBuffer[A], ordering: Ordering[A]) extends SortedSet[A](items, ordering):
  override def incl(elem: A): TreeSet[A] = unsafeCast(super.incl(elem))
  override def excl(elem: A): TreeSet[A] = unsafeCast(super.excl(elem))
  override def ++(that: IterableOnce[A]): TreeSet[A] = unsafeCast(super.++(that))
  override def concat(that: IterableOnce[A]): TreeSet[A] = unsafeCast(super.concat(that))
  override def removedAll(that: IterableOnce[A]): TreeSet[A] = unsafeCast(super.removedAll(that))
  override def +(elem: A): TreeSet[A] = incl(elem)
  override def -(elem: A): TreeSet[A] = excl(elem)

object TreeSet:
  def empty[A](implicit ord: Ordering[A]): TreeSet[A] = new TreeSet(emptyBuffer[A], ord)
  def apply[A](elems: A*)(implicit ord: Ordering[A]): TreeSet[A] = from(elems)
  def from[A](elems: IterableOnce[A])(implicit ord: Ordering[A]): TreeSet[A] = SortedSet.from(elems)

// Insertion-ordered, as scala-library's ListSet iterates: an array with the elements in the
// order they came, which an update copies like the other sets do.
final class ListSet[A](private val items: RawBuffer[A]) extends SetOps[A, ListSet[A]], IterableOps[A, Set, ListSet[A]], Set[A]:
  override def buildC(elems: RawBuffer[A]): ListSet[A] = ListSet.from(Vector.wrap(elems))
  override def iterator: Iterator[A] = Iterator.over(items)
  @jvmEvidence
  @jvm("rt $0 $1 rtcall arrayOf(Lscala/IterableOnce;Lscala/reflect/ClassTag;)Ljava/lang/Object; cast_result")
  override def toArray[B >: A: scala.reflect.ClassTag]: Array[B] = unsafeCast(copyArray(items))
  override def size: Int = items.length
  override def knownSize: Int = items.length
  override def isEmpty: Boolean = items.length == 0
  def foreach[U](f: A => U): Unit = items.foreach(f)
  def contains(elem: A): Boolean = items.exists(_ == elem)
  def incl(elem: A): ListSet[A] =
    if contains(elem) then this
    else
      val copy = copyArray(items)
      copy.push(elem)
      new ListSet(copy)
  def excl(elem: A): ListSet[A] = if contains(elem) then filter(_ != elem) else this
  override def ++(that: IterableOnce[A]): ListSet[A] = concat(that)
  override def concat(that: IterableOnce[A]): ListSet[A] =
    val all = copyArray(items)
    that.foreach(x => all.push(x))
    ListSet.from(Vector.wrap(all))
  def removedAll(that: IterableOnce[A]): ListSet[A] =
    val dropped = Set.from(that)
    filter(x => !dropped.contains(x))
  override def +(elem: A): ListSet[A] = incl(elem)
  override def -(elem: A): ListSet[A] = excl(elem)
  override def collectionClassName: String = "ListSet"
  override def toString: String = mkString("ListSet(", ", ", ")")

object ListSet:
  def empty[A]: ListSet[A] = new ListSet(emptyBuffer[A])
  def apply[A](elems: A*): ListSet[A] = from(elems)
  def from[A](elems: IterableOnce[A]): ListSet[A] =
    val out = emptyBuffer[A]
    elems.iterator.foreach(x => if !out.exists(_ == x) then out.push(x))
    new ListSet(out)
  def newBuilder[A]: scala.collection.mutable.Builder[A, ListSet[A]] =
    scala.collection.mutable.ArrayBuffer.empty[A].mapResult(items => from(items))

// Map keeps insertion order and an update of a key keeps its position, as ListMap does.
// The std's `Map` under its scala-library name, below `collection.Map` as the mutable one is.
export scala.Map

// The std's `ArraySeq` under the name scala-library gives it, for the files that serve both
// standard libraries (`IArray`).
type ArraySeq[+A] = scala.ArraySeq[A]

object ArraySeq:
  def apply[A](elems: A*): scala.ArraySeq[A] = scala.ArraySeq(elems*)
  def empty[A]: scala.ArraySeq[A] = scala.ArraySeq.empty[A]
  def from[A](source: IterableOnce[A]): scala.ArraySeq[A] = scala.ArraySeq.from(source)
  def unsafeWrapArray[A](x: Array[A]): scala.ArraySeq[A] = scala.ArraySeq.unsafeWrapArray(x)

// A queue over an array copied on each update, as the other immutable sequences here are:
// scala-library's two-list queue has the same operations and order.
final class Queue[+A](private val items: RawBuffer[A]) extends SeqOps[A, Queue, Queue[A]], Seq[A]:
  def buildCC[B](items: RawBuffer[B]): Queue[B] = new Queue(items)
  override def iterator: Iterator[A] = Iterator.over(items)
  def length: Int = items.length
  def apply(i: Int): A =
    if i < 0 || i >= items.length then indexOutOfBounds(i.toString)
    else items(i)
  def foreach[U](f: A => U): Unit =
    var i = 0
    while i < items.length do
      f(items(i))
      i += 1
  def enqueue[B >: A](elem: B): Queue[B] = this :+ elem
  def enqueueAll[B >: A](iter: Iterable[B]): Queue[B] = this ++ iter
  def dequeue: (A, Queue[A]) =
    if items.length == 0 then throw new NoSuchElementException("dequeue on empty queue")
    else (items(0), new Queue(items.drop(1)))
  def dequeueOption: Option[(A, Queue[A])] = if items.length == 0 then None else Some(dequeue)
  def front: A = head
  @jvmEvidence
  @jvm("rt $0 $1 rtcall arrayOf(Lscala/IterableOnce;Lscala/reflect/ClassTag;)Ljava/lang/Object; cast_result")
  override def toArray[B >: A: scala.reflect.ClassTag]: Array[B] = unsafeCast(copyArray(items))
  override def collectionClassName: String = "Queue"
  override def toString: String = mkString("Queue(", ", ", ")")

object Queue:
  def apply[A](elems: A*): Queue[A] = new Queue(iterableToArray(elems))
  def empty[A]: Queue[A] = new Queue(emptyBuffer[A])
  def from[A](source: IterableOnce[A]): Queue[A] = new Queue(iterableToArray(source))

// ListMap is a class of its own over the std's insertion-ordered Map: a library's givens per
// map class (zio-json's `map` and `listMap`) need the two types distinct.
final class ListMap[K, +V](private val underlying: Map[K, V]) extends Map[K, V]:
  override def size: Int = underlying.size
  override def knownSize: Int = underlying.knownSize
  override def isEmpty: Boolean = underlying.isEmpty
  override def contains(key: K): Boolean = underlying.contains(key)
  def get(key: K): Option[V] = underlying.get(key)
  def iterator: Iterator[(K, V)] = underlying.iterator
  override def foreach[U](f: ((K, V)) => U): Unit = underlying.foreach(f)
  override def buildC(items: RawBuffer[(K, V)]): ListMap[K, V] = ListMap.wrap(Map.from(Vector.wrap(items)))
  def updated[V1 >: V](key: K, value: V1): ListMap[K, V1] = new ListMap(underlying.updated(key, value))
  def removed(key: K): ListMap[K, V] = if underlying.contains(key) then ListMap.wrap(underlying.removed(key)) else this
  override def removedAll(keys: IterableOnce[K]): ListMap[K, V] = ListMap.wrap(underlying.removedAll(keys))
  override def ++[V1 >: V](that: IterableOnce[(K, V1)]): ListMap[K, V1] = ListMap.wrap(underlying ++ that)
  override def +[V1 >: V](entry: (K, V1)): ListMap[K, V1] = updated(entry._1, entry._2)
  override def -(key: K): ListMap[K, V] = removed(key)
  override def --(keys: IterableOnce[K]): ListMap[K, V] = removedAll(keys)
  override def updatedWith[V1 >: V](key: K)(remap: Option[V] => Option[V1]): ListMap[K, V1] = ListMap.wrap(underlying.updatedWith(key)(remap))
  override def filter(p: ((K, V)) => Boolean): ListMap[K, V] = ListMap.wrap(underlying.filter(p))
  override def filterNot(p: ((K, V)) => Boolean): ListMap[K, V] = ListMap.wrap(underlying.filterNot(p))
  override def partition(p: ((K, V)) => Boolean): (ListMap[K, V], ListMap[K, V]) = (filter(p), filterNot(p))
  override def withFilter(p: ((K, V)) => Boolean): ListMap[K, V] = filter(p)
  override def take(n: Int): ListMap[K, V] = ListMap.wrap(underlying.take(n))
  override def drop(n: Int): ListMap[K, V] = ListMap.wrap(underlying.drop(n))
  override def takeRight(n: Int): ListMap[K, V] = ListMap.wrap(underlying.takeRight(n))
  override def dropRight(n: Int): ListMap[K, V] = ListMap.wrap(underlying.dropRight(n))
  override def slice(from: Int, until: Int): ListMap[K, V] = ListMap.wrap(underlying.slice(from, until))
  override def tail: ListMap[K, V] = ListMap.wrap(underlying.tail)
  override def init: ListMap[K, V] = ListMap.wrap(underlying.init)
  override def keys: List[K] = underlying.keys
  override def values: List[V] = underlying.values
  @jvmWide
  override def map[K2, V2](f: ((K, V)) => (K2, V2)): Map[K2, V2] = ListMap.wrapResults(underlying.map(f))
  @jvmWide
  override def collect[K2, V2](pf: PartialFunction[(K, V), (K2, V2)]): Map[K2, V2] = ListMap.wrapResults(underlying.collect(pf))
  @jvmWide
  override def flatMap[K2, V2](f: ((K, V)) => IterableOnce[(K2, V2)]): Map[K2, V2] = ListMap.wrapResults(underlying.flatMap(f))
  override def collectionClassName: String = "ListMap"
  override def toString: String = mkString("ListMap(", ", ", ")")

object ListMap extends MapFactory[ListMap]:
  private val emptyListMap: ListMap[Nothing, Nothing] = new ListMap(Map.empty)
  def empty[K, V]: ListMap[K, V] = unsafeCast(emptyListMap)
  def apply[K, V](entries: (K, V)*): ListMap[K, V] = from(entries)
  def from[K, V](entries: IterableOnce[(K, V)]): ListMap[K, V] = entries match
    case m: ListMap[?, ?] => unsafeCast(m)
    case _ => wrap(Map.from(entries))
  // The empty map is the shared instance, as in Scala.
  def wrap[K, V](m: Map[K, V]): ListMap[K, V] = if m.isEmpty then empty else new ListMap(m)
  // Reached through Iterable, a transform may give a List rather than a Map (`Map.fromResults`).
  def wrapResults[K, V](m: Map[K, V]): Map[K, V] = m match
    case m: Map[?, ?] => wrap(unsafeCast(m))
    case _ => m
  def newBuilder[K, V]: scala.collection.mutable.ReusableBuilder[(K, V), ListMap[K, V]] =
    scala.collection.mutable.ArrayBuffer.empty[(K, V)].mapResult(from)

// A sorted array of entries: lookups are binary searches, updates copy the array like SortedSet.
class SortedMap[K, +V](private val entries: RawBuffer[(K, Any)], val ordering: Ordering[K]) extends IterableOps[(K, V), Iterable, SortedMap[K, V]], Iterable[(K, V)]:
  def buildCC[B](items: RawBuffer[B]): Iterable[B] = fromArray(items)
  override def buildC(items: RawBuffer[(K, V)]): SortedMap[K, V] = SortedMap.from(Vector.wrap(items))(using ordering)
  override def iterator: Iterator[(K, V)] = Iterator.over(unsafeCast(entries))
  @jvmEvidence
  @jvm("rt $0 $1 rtcall arrayOf(Lscala/IterableOnce;Lscala/reflect/ClassTag;)Ljava/lang/Object; cast_result")
  override def toArray[B >: (K, V): scala.reflect.ClassTag]: Array[B] = unsafeCast(copyArray(entries))
  override def size: Int = entries.length
  override def knownSize: Int = entries.length
  override def isEmpty: Boolean = entries.length == 0
  def foreach[U](f: ((K, V)) => U): Unit = entries.foreach(e => f(unsafeCast(e)))
  // The index of key, or -(insertion point) - 1 without one.
  private def search(key: K): Int =
    var lo = 0
    var hi = entries.length - 1
    var found = -1
    while found < 0 && lo <= hi do
      val mid = (lo + hi) / 2
      val c = ordering.compare(entries(mid)._1, key)
      if c == 0 then found = mid
      else if c < 0 then lo = mid + 1
      else hi = mid - 1
    if found >= 0 then found else -lo - 1
  def contains(key: K): Boolean = search(key) >= 0
  def isDefinedAt(key: K): Boolean = contains(key)
  def get(key: K): Option[V] =
    val i = search(key)
    if i >= 0 then Some(unsafeCast(entries(i)._2)) else None
  def apply(key: K): V = get(key) match
    case Some(v) => v
    case None => noSuchElement("key not found: " + key.toString)
  def getOrElse[V1 >: V](key: K, default: => V1): V1 = get(key).getOrElse(default)
  def updated[V1 >: V](key: K, value: V1): SortedMap[K, V1] =
    val i = search(key)
    val copy = copyArray(entries)
    if i >= 0 then copy(i) = (key, value)
    else scala.collection.mutable.bufferInsert(copy, -i - 1, (key, value))
    new TreeMap(copy, ordering)
  def +[V1 >: V](entry: (K, V1)): SortedMap[K, V1] = updated(entry._1, entry._2)
  def removed(key: K): SortedMap[K, V] =
    val i = search(key)
    if i < 0 then this
    else
      val copy = copyArray(entries)
      scala.collection.mutable.bufferRemove(copy, i)
      new TreeMap(copy, ordering)
  def -(key: K): SortedMap[K, V] = removed(key)
  def ++[V1 >: V](that: IterableOnce[(K, V1)]): SortedMap[K, V1] =
    var out: SortedMap[K, V1] = this
    that.foreach(e => out = out.updated(e._1, e._2))
    out
  override def concat[V1 >: V](that: IterableOnce[(K, V1)]): SortedMap[K, V1] = this ++ that
  def --(keys: IterableOnce[K]): SortedMap[K, V] =
    var out: SortedMap[K, V] = this
    keys.foreach(k => out = out.removed(k))
    out
  def keys: SortedSet[K] = keySet
  def values: List[V] = fromArray(entries.map(e => unsafeCast(e._2)))
  def keySet: SortedSet[K] = new TreeSet(entries.map(e => e._1), ordering)
  def keysIterator: Iterator[K] = Iterator.over(entries.map(e => e._1))
  def valuesIterator: Iterator[V] = Iterator.over(entries.map(e => unsafeCast(e._2)))
  def firstKey: K = head._1
  def lastKey: K = last._1
  // A function over the entries that gives pairs builds a sorted map, any other an Iterable,
  // as scala-library's overloads do; called through the static type Iterable, no ordering is
  // passed and the result is a plain Map.
  def map[K2, V2](f: ((K, V)) => (K2, V2))(implicit ord: Ordering[K2]): SortedMap[K2, V2] =
    SortedMap.sortedOrPlain(rawItems(this).map(f), ord)
  override def map[B](f: ((K, V)) => B): Iterable[B] = fromArray(rawItems(this).map(f))
  def collect[K2, V2](pf: PartialFunction[(K, V), (K2, V2)])(implicit ord: Ordering[K2]): SortedMap[K2, V2] =
    SortedMap.sortedOrPlain(rawItems(this).collect(pf), ord)
  override def collect[B](pf: PartialFunction[(K, V), B]): Iterable[B] = fromArray(Vector.wrap(rawItems(this)).collect(pf).unsafeArray)
  def flatMap[K2, V2](f: ((K, V)) => IterableOnce[(K2, V2)])(implicit ord: Ordering[K2]): SortedMap[K2, V2] =
    val out = emptyBuffer[(K2, V2)]
    foreach(e => f(e).iterator.foreach(x => out.push(x)))
    SortedMap.sortedOrPlain(out, ord)
  override def flatMap[B](f: ((K, V)) => IterableOnce[B]): Iterable[B] = fromArray(Vector.wrap(rawItems(this)).flatMap(f).unsafeArray)
  def mapValues[W](f: V => W): SortedMap[K, W] = new TreeMap(entries.map(e => (e._1, f(unsafeCast(e._2)))), ordering)
  def transform[W](f: (K, V) => W): SortedMap[K, W] = new TreeMap(entries.map(e => (e._1, f(e._1, unsafeCast(e._2)))), ordering)
  def filterKeys(p: K => Boolean): SortedMap[K, V] = filter(e => p(e._1))
  def rangeFrom(from: K): SortedMap[K, V] = filter(e => ordering.gteq(e._1, from))
  def rangeUntil(until: K): SortedMap[K, V] = filter(e => ordering.lt(e._1, until))
  def range(from: K, until: K): SortedMap[K, V] = filter(e => ordering.gteq(e._1, from) && ordering.lt(e._1, until))
  override def toMap[K2, V2](implicit ev: ((K, V)) <:< (K2, V2)): scala.Map[K2, V2] = unsafeCast(scala.Map.from(this))
  override def mkString(start: String, sep: String, end: String): String =
    joinStrings(Vector.wrap(entries.map(e => e._1.toString + " -> " + e._2.toString)), start, sep, end)
  def equals(that: Any): Boolean = that match
    case m: SortedMap[?, ?] => scala.Map.from(this).equals(scala.Map.from(m))
    case m: scala.Map[?, ?] => scala.Map.from(this).equals(m)
    case _ => false
  override def hashCode: Int = scala.Map.from(this).hashCode
  override def collectionClassName: String = "TreeMap"
  override def toString: String = mkString("TreeMap(", ", ", ")")

object SortedMap:
  def sortedOrPlain[K, V](entries: RawBuffer[(K, V)], ord: Ordering[K]): SortedMap[K, V] =
    if js.isUndefined(ord) then unsafeCast(scala.Map.from(Vector.wrap(entries))) else from(Vector.wrap(entries))(using ord)
  def empty[K, V](implicit ord: Ordering[K]): SortedMap[K, V] = new TreeMap(emptyBuffer[(K, Any)], ord)
  def apply[K, V](entries: (K, V)*)(implicit ord: Ordering[K]): SortedMap[K, V] = from(entries)
  def from[K, V](entries: IterableOnce[(K, V)])(implicit ord: Ordering[K]): SortedMap[K, V] =
    var out: SortedMap[K, V] = empty[K, V]
    entries.foreach(e => out = out.updated(e._1, e._2))
    out
  def newBuilder[K, V](implicit ord: Ordering[K]): scala.collection.mutable.Builder[(K, V), SortedMap[K, V]] =
    scala.collection.mutable.ArrayBuffer.empty[(K, V)].mapResult(items => from(items))

// Every sorted map is a TreeMap, as in scala-library; the class keeps the two types distinct.
final class TreeMap[K, +V](entries: RawBuffer[(K, Any)], ordering: Ordering[K]) extends SortedMap[K, V](entries, ordering):
  override def updated[V1 >: V](key: K, value: V1): TreeMap[K, V1] = unsafeCast(super.updated(key, value))
  override def +[V1 >: V](entry: (K, V1)): TreeMap[K, V1] = unsafeCast(super.+(entry))
  override def removed(key: K): TreeMap[K, V] = unsafeCast(super.removed(key))
  override def -(key: K): TreeMap[K, V] = unsafeCast(super.-(key))
  override def ++[V1 >: V](that: IterableOnce[(K, V1)]): TreeMap[K, V1] = unsafeCast(super.++(that))
  override def concat[V1 >: V](that: IterableOnce[(K, V1)]): TreeMap[K, V1] = unsafeCast(super.concat(that))
  override def --(keys: IterableOnce[K]): TreeMap[K, V] = unsafeCast(super.--(keys))

object TreeMap:
  def empty[K, V](implicit ord: Ordering[K]): TreeMap[K, V] = new TreeMap(emptyBuffer[(K, Any)], ord)
  def apply[K, V](entries: (K, V)*)(implicit ord: Ordering[K]): TreeMap[K, V] = from(entries)
  def from[K, V](entries: IterableOnce[(K, V)])(implicit ord: Ordering[K]): TreeMap[K, V] = unsafeCast(SortedMap.from(entries))

// IntMap is a Map keyed by Int and prints as one.
type IntMap[+V] = scala.Map[Int, V]

object IntMap:
  def empty[V]: scala.Map[Int, V] = scala.Map.empty[Int, V]
  def apply[V](entries: (Int, V)*): scala.Map[Int, V] = scala.Map.from(entries)
  def from[V](entries: IterableOnce[(Int, V)]): scala.Map[Int, V] = scala.Map.from(entries)

/** scala-library's `VectorBuilder`, what `Vector.newBuilder` gives, over the array it builds. */
final class VectorBuilder[A] extends scala.collection.mutable.ReusableBuilder[A, Vector[A]]:
  private var items: RawBuffer[A] = emptyBuffer[A]
  def addOne(elem: A): this.type =
    items.push(elem)
    this
  override def addAll(elems: IterableOnce[A]): this.type =
    elems.foreach(e => items.push(e))
    this
  def result(): Vector[A] = Vector.wrap(copyArray(items))
  def clear(): Unit = items = emptyBuffer[A]
  override def knownSize: Int = items.length
  def size: Int = items.length
  def isEmpty: Boolean = items.length == 0
  def nonEmpty: Boolean = items.length != 0
