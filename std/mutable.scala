package scala.collection.mutable

/** What a collection is built up with: `scala.collection.mutable.Builder` as library bodies name it. */
trait Builder[-A, +To]:
  def addOne(elem: A): this.type
  def +=(elem: A): this.type = addOne(elem)
  def addAll(elems: IterableOnce[A]): this.type =
    elems.foreach(e => addOne(e))
    this
  def ++=(elems: IterableOnce[A]): this.type = addAll(elems)
  def result(): To
  def clear(): Unit
  def sizeHint(size: Int): Unit = ()
  def sizeHint(coll: IterableOnce[?], delta: Int = 0): Unit = ()
  def knownSize: Int = -1
  def mapResult[NewTo](f: To => NewTo): ReusableBuilder[A, NewTo] =
    val self = this
    new ReusableBuilder[A, NewTo]:
      def addOne(elem: A): this.type =
        self.addOne(elem)
        this
      def result(): NewTo = f(self.result())
      def clear(): Unit = self.clear()

/** A builder whose `result()` can be called again after more elements, as scala-library's. */
trait ReusableBuilder[-A, +To] extends Builder[A, To]

final class ArrayBuffer[A](private val items: RawBuffer[A]) extends SeqOps[A, ArrayBuffer, ArrayBuffer[A]], IndexedSeq[A], Builder[A, ArrayBuffer[A]]:
  def this() = this(emptyBuffer[A])
  def this(initialSize: Int) = this(emptyBuffer[A])
  def buildCC[B](items: RawBuffer[B]): ArrayBuffer[B] = new ArrayBuffer(items)
  override def iterator: Iterator[A] = Iterator.over(items)
  def length: Int = items.length
  def apply(i: Int): A =
    if i < 0 || i >= items.length then indexOutOfBounds(outOfBounds(i))
    else items(i)
  def update(i: Int, value: A): Unit =
    if i < 0 || i >= items.length then indexOutOfBounds(outOfBounds(i))
    else items(i) = value
  private def outOfBounds(i: Int): String = i.toString + " is out of bounds (min 0, max " + (items.length - 1).toString + ")"
  def +=(value: A): this.type =
    items.push(value)
    this
  def +=(value1: A, value2: A, values: A*): this.type =
    items.push(value1)
    items.push(value2)
    this ++= values
  def append(value: A): this.type = this += value
  inline def append(values: A*): this.type = this ++= values
  def addOne(value: A): this.type = this += value
  def ++=(values: IterableOnce[A]): this.type =
    values.foreach(v => items.push(v))
    this
  def addAll(values: IterableOnce[A]): this.type = this ++= values
  def appendAll(values: IterableOnce[A]): ArrayBuffer[A] = this ++= values
  def -=(value: A): ArrayBuffer[A] =
    val i = indexOf(value)
    if i >= 0 then bufferRemove(items, i)
    this
  def subtractOne(value: A): ArrayBuffer[A] = this -= value
  def prependAll(values: IterableOnce[A]): ArrayBuffer[A] =
    bufferInsertAll(items, 0, iterableToArray(values))
    this
  def insertAll(index: Int, values: IterableOnce[A]): Unit = bufferInsertAll(items, index, iterableToArray(values))
  def filterInPlace(p: A => Boolean): ArrayBuffer[A] =
    val kept = items.filter(p)
    bufferClear(items)
    kept.foreach(x => items.push(x))
    this
  def mapInPlace(f: A => A): ArrayBuffer[A] =
    var i = 0
    while i < items.length do
      items(i) = f(items(i))
      i += 1
    this
  def sortInPlace[B >: A]()(implicit ord: Ordering[B]): ArrayBuffer[A] =
    sortArray(items, (a, b) => ord.compare(a, b))
    this
  def sortInPlaceBy[B](f: A => B)(implicit ord: Ordering[B]): ArrayBuffer[A] =
    sortArray(items, (a, b) => ord.compare(f(a), f(b)))
    this
  def dropInPlace(n: Int): ArrayBuffer[A] =
    bufferRemoveRange(items, 0, n)
    this
  def dropRightInPlace(n: Int): ArrayBuffer[A] =
    bufferRemoveRange(items, Math.max(items.length - n, 0), n)
    this
  def takeInPlace(n: Int): ArrayBuffer[A] = dropRightInPlace(items.length - Math.max(n, 0))
  def result(): ArrayBuffer[A] = this
  override def clone(): ArrayBuffer[A] = new ArrayBuffer(copyArray(items))
  def prepend(value: A): ArrayBuffer[A] =
    bufferInsert(items, 0, value)
    this
  def insert(index: Int, value: A): Unit = bufferInsert(items, index, value)
  def remove(index: Int): A =
    val removed = apply(index)
    bufferRemove(items, index)
    removed
  def clear(): Unit = bufferClear(items)
  def head: A = if items.length == 0 then noSuchElement("head of empty ArrayBuffer") else apply(0)
  def last: A = if items.length == 0 then noSuchElement("last of empty ArrayBuffer") else apply(items.length - 1)
  def foreach[U](f: A => U): Unit =
    var i = 0
    while i < items.length do
      f(items(i))
      i += 1
  @jvmEvidence
  @jvm("rt $0 $1 rtcall arrayOf(Lscala/IterableOnce;Lscala/reflect/ClassTag;)Ljava/lang/Object; cast_result")
  override def toArray[B >: A: scala.reflect.ClassTag]: Array[B] = unsafeCast(copyArray(items))
  override def toSeq: Seq[A] = toList
  override def map[B](f: A => B): ArrayBuffer[B] = new ArrayBuffer(items.map(f))
  override def filter(p: A => Boolean): ArrayBuffer[A] = new ArrayBuffer(items.filter(p))
  def sortInPlaceWith(lt: (A, A) => Boolean): ArrayBuffer[A] =
    sortArray(items, (a, b) => if lt(a, b) then -1 else if lt(b, a) then 1 else 0)
    this
  override def collectionClassName: String = "ArrayBuffer"
  override def toString: String = mkString("ArrayBuffer(", ", ", ")")

@js("void $0.splice($1, 0, $2)")
@jvm("$0 $1:I $2:L invokevirtual java/util/ArrayList.add(ILjava/lang/Object;)V")
def bufferInsert[A](items: RawBuffer[A], index: Int, value: A): Unit

@js("void $0.splice($1, 1)")
@jvm("$0 $1:I invokevirtual java/util/ArrayList.remove(I)Ljava/lang/Object; pop")
def bufferRemove[A](items: RawBuffer[A], index: Int): Unit

@js("void ($0.length = 0)")
@jvm("invokevirtual java/util/ArrayList.clear()V")
def bufferClear[A](items: RawBuffer[A]): Unit

@js("void $0.splice($1, 0, ...$2)")
@jvm("$0 $1:I $2 invokevirtual java/util/ArrayList.addAll(ILjava/util/Collection;)Z pop")
def bufferInsertAll[A](items: RawBuffer[A], index: Int, values: RawBuffer[A]): Unit

@js("void $0.splice($1, $2)")
def bufferRemoveRange[A](items: RawBuffer[A], index: Int, count: Int): Unit = scala.runtime.removeRange(items, index, index + count)

object ArrayBuffer:
  def empty[A]: ArrayBuffer[A] = new ArrayBuffer(emptyBuffer[A])
  def newBuilder[A]: Builder[A, ArrayBuffer[A]] = empty[A].mapResult(b => b)
  def apply[A](elems: A*): ArrayBuffer[A] = from(elems)
  def from[A](elems: IterableOnce[A]): ArrayBuffer[A] = new ArrayBuffer(iterableToArray(elems))
  def fill[A](n: Int)(elem: => A): ArrayBuffer[A] = new ArrayBuffer(filledBuffer(n)(elem))
  def tabulate[A](n: Int)(f: Int => A): ArrayBuffer[A] = new ArrayBuffer(tabulatedBuffer(n)(f))

type Buffer[A] = ArrayBuffer[A]
/** scala-library's `ListBuffer`: a buffer that builds a `List`, over an `ArrayBuffer`. */
final class ListBuffer[A](private val buf: ArrayBuffer[A]) extends SeqOps[A, ListBuffer, ListBuffer[A]], IndexedSeq[A], Builder[A, List[A]]:
  def this() = this(ArrayBuffer.empty[A])
  def buildCC[B](items: RawBuffer[B]): ListBuffer[B] = new ListBuffer(new ArrayBuffer(items))
  override def iterator: Iterator[A] = buf.iterator
  def foreach[U](f: A => U): Unit = buf.foreach(f)
  def length: Int = buf.length
  def apply(i: Int): A = buf(i)
  def update(i: Int, value: A): Unit = buf(i) = value
  def +=(value: A): this.type =
    buf += value
    this
  def +=(value1: A, value2: A, values: A*): this.type =
    buf += value1
    buf += value2
    buf ++= values
    this
  def append(value: A): this.type = this += value
  inline def append(values: A*): this.type = this ++= values
  def addOne(value: A): this.type = this += value
  def ++=(values: IterableOnce[A]): this.type =
    buf ++= values
    this
  override def addAll(values: IterableOnce[A]): this.type = this ++= values
  def appendAll(values: IterableOnce[A]): ListBuffer[A] = this ++= values
  def -=(value: A): ListBuffer[A] =
    buf -= value
    this
  def subtractOne(value: A): ListBuffer[A] = this -= value
  def +=:(value: A): ListBuffer[A] =
    buf.prepend(value)
    this
  def prepend(value: A): ListBuffer[A] = value +=: this
  def prependAll(values: IterableOnce[A]): ListBuffer[A] =
    buf.prependAll(values)
    this
  def insert(index: Int, value: A): Unit = buf.insert(index, value)
  def insertAll(index: Int, values: IterableOnce[A]): Unit = buf.insertAll(index, values)
  def remove(index: Int): A = buf.remove(index)
  def clear(): Unit = buf.clear()
  def result(): List[A] = buf.toList
  override def toList: List[A] = buf.toList
  def mapInPlace(f: A => A): ListBuffer[A] =
    buf.mapInPlace(f)
    this
  def filterInPlace(p: A => Boolean): ListBuffer[A] =
    buf.filterInPlace(p)
    this
  override def collectionClassName: String = "ListBuffer"
  override def toString: String = mkString("ListBuffer(", ", ", ")")

object Buffer:
  def empty[A]: ArrayBuffer[A] = ArrayBuffer.empty[A]
  def apply[A](elems: A*): ArrayBuffer[A] = ArrayBuffer.from(elems)

object ListBuffer:
  def empty[A]: ListBuffer[A] = new ListBuffer(ArrayBuffer.empty[A])
  def apply[A](elems: A*): ListBuffer[A] = new ListBuffer(ArrayBuffer.from(elems))
  def from[A](source: IterableOnce[A]): ListBuffer[A] = new ListBuffer(ArrayBuffer.from(source))
  def newBuilder[A]: Builder[A, ListBuffer[A]] = ArrayBuffer.empty[A].mapResult(b => new ListBuffer(b))

class HashMap[K, V](private val raw: RawMap[K, V] = newRawMap[K, V]) extends IterableOps[(K, V), Iterable, HashMap[K, V]], scala.collection.Map[K, V]:
  def buildCC[B](items: RawBuffer[B]): Iterable[B] = fromArray(items)
  // A mutable map is its own builder, as scala-library's `MapOps` is.
  def result(): HashMap[K, V] = this
  override def buildC(items: RawBuffer[(K, V)]): HashMap[K, V] = HashMap.from(Vector.wrap(items))
  override def size: Int = raw.rawSize
  override def knownSize: Int = raw.rawSize
  override def isEmpty: Boolean = raw.rawSize == 0
  override def contains(key: K): Boolean = raw.rawHas(key)
  def get(key: K): Option[V] = if raw.rawHas(key) then Some(raw.rawGet(key)) else None
  override def apply(key: K): V =
    if raw.rawHas(key) then raw.rawGet(key)
    else noSuchElement("key not found: " + key.toString)
  override def getOrElse[V1 >: V](key: K, default: => V1): V1 = if raw.rawHas(key) then raw.rawGet(key) else default
  def getOrElseUpdate(key: K, default: => V): V =
    if raw.rawHas(key) then raw.rawGet(key)
    else
      val value = default
      raw.rawSet(key, value)
      value
  def update(key: K, value: V): Unit = raw.rawSet(key, value)
  def put(key: K, value: V): Option[V] =
    val previous = get(key)
    raw.rawSet(key, value)
    previous
  def +=(entry: (K, V)): this.type =
    raw.rawSet(entry._1, entry._2)
    this
  def addOne(entry: (K, V)): this.type = this += entry
  def ++=(entries: IterableOnce[(K, V)]): this.type =
    entries.foreach(e => raw.rawSet(e._1, e._2))
    this
  def addAll(entries: IterableOnce[(K, V)]): this.type = this ++= entries
  def -=(key: K): HashMap[K, V] =
    raw.rawDelete(key)
    this
  def subtractOne(key: K): HashMap[K, V] = this -= key
  def --=(keys: IterableOnce[K]): HashMap[K, V] =
    keys.foreach(k => raw.rawDelete(k))
    this
  def updateWith(key: K)(remap: Option[V] => Option[V]): Option[V] =
    val next = remap(get(key))
    next match
      case Some(v) => raw.rawSet(key, v)
      case None => raw.rawDelete(key)
    next
  def filterInPlace(p: (K, V) => Boolean): HashMap[K, V] =
    raw.rawKeys.foreach(k => if !p(k, raw.rawGet(k)) then raw.rawDelete(k))
    this
  def mapValues[W](f: V => W): MapView[K, W] = view.mapValues(f)
  def withDefault(d: K => V): HashMap[K, V] = new Map.WithDefault(raw, d)
  def withDefaultValue(d: V): HashMap[K, V] = new Map.WithDefault(raw, _ => d)
  def mapValuesInPlace(f: (K, V) => V): HashMap[K, V] =
    raw.rawKeys.foreach(k => raw.rawSet(k, f(k, raw.rawGet(k))))
    this
  def remove(key: K): Option[V] =
    val previous = get(key)
    raw.rawDelete(key)
    previous
  def clear(): Unit = raw.rawClear()
  override def clone(): HashMap[K, V] = HashMap.from(this)
  def keys: List[K] = fromArray(raw.rawKeys)
  def values: List[V] = fromArray(raw.rawValues)
  def keySet: scala.Set[K] = scala.Set.from(Vector.wrap(raw.rawKeys))
  def keysIterator: Iterator[K] = Iterator.over(raw.rawKeys)
  def valuesIterator: Iterator[V] = Iterator.over(raw.rawValues)
  override def toMap[K2, V2](implicit ev: ((K, V)) <:< (K2, V2)): scala.Map[K2, V2] = unsafeCast(scala.Map.from(this))
  def foreach[U](f: ((K, V)) => U): Unit = raw.rawForeach((k, v) => f((k, v)))
  def foreachEntry[U](f: (K, V) => U): Unit = raw.rawForeach(f)
  // A function over the entries that gives pairs builds a HashMap, any other an Iterable, as
  // scala-library's overloads of `MapOps` and `IterableOps` do.
  @jvmWide
  def map[K2, V2](f: ((K, V)) => (K2, V2)): HashMap[K2, V2] = HashMap.from(Vector.wrap(rawItems(this).map(f)))
  override def map[B](f: ((K, V)) => B): Iterable[B] = fromArray(rawItems(this).map(f))
  @jvmWide
  def collect[K2, V2](pf: PartialFunction[(K, V), (K2, V2)]): HashMap[K2, V2] =
    HashMap.from(Vector.wrap(rawItems(this)).collect(pf))
  override def collect[B](pf: PartialFunction[(K, V), B]): Iterable[B] = fromArray(Vector.wrap(rawItems(this)).collect(pf).unsafeArray)
  @jvmWide
  def flatMap[K2, V2](f: ((K, V)) => IterableOnce[(K2, V2)]): HashMap[K2, V2] =
    HashMap.from(Vector.wrap(rawItems(this)).flatMap(f))
  override def flatMap[B](f: ((K, V)) => IterableOnce[B]): Iterable[B] = fromArray(Vector.wrap(rawItems(this)).flatMap(f).unsafeArray)
  override def mkString(start: String, sep: String, end: String): String = scala.Map.from(this).mkString(start, sep, end)
  override def view: MapView[K, V] = new MapView(() => iterator, k => get(k))
  def equals(that: Any): Boolean = that match
    case m: scala.Map[?, ?] =>
      val other: scala.Map[K, Any] = unsafeCast(m)
      size == other.size && raw.rawKeys.forall(k => other.get(k) match
        case Some(v) => v == raw.rawGet(k)
        case None => false)
    case m: HashMap[?, ?] => rawMapEquals(unsafeCast(raw), unsafeCast(m.raw))
    case _ => false
  override def hashCode: Int = rawMapHash(unsafeCast(raw))
  override def collectionClassName: String = "HashMap"
  override def toString: String = mkString("HashMap(", ", ", ")")

object HashMap:
  def empty[K, V]: HashMap[K, V] = new HashMap(newRawMap[K, V])
  def apply[K, V](entries: (K, V)*): HashMap[K, V] = from(entries)
  def from[K, V](entries: IterableOnce[(K, V)]): HashMap[K, V] =
    val m = empty[K, V]
    entries.foreach(e => m.update(e._1, e._2))
    m

type Map[K, V] = HashMap[K, V]

// scala-library's deprecated `MultiMap`, mixed into a map of sets: `new HashMap[K, Set[V]] with
// MultiMap[K, V]`.
trait MultiMap[K, V]:
  self: HashMap[K, HashSet[V]] =>
  protected def makeSet: HashSet[V] = HashSet.empty[V]
  def addBinding(key: K, value: V): this.type =
    get(key) match
      case Some(set) => set += value
      case None =>
        val set = makeSet
        set += value
        update(key, set)
    this
  def removeBinding(key: K, value: V): this.type =
    get(key) match
      case Some(set) =>
        set -= value
        if set.isEmpty then remove(key)
      case None =>
    this
  def entryExists(key: K, p: V => Boolean): Boolean = get(key) match
    case Some(set) => set.exists(p)
    case None => false

object Map:
  /** A view of a map whose `apply` answers a missing key with the default; it shares the map's
    * entries, so an update through either is seen by both.
    */
  final class WithDefault[K, V](entries: RawMap[K, V], defaultValue: K => V) extends HashMap[K, V](entries):
    override def apply(key: K): V = getOrElse(key, defaultValue(key))
    def default(key: K): V = defaultValue(key)
  def empty[K, V]: HashMap[K, V] = HashMap.empty[K, V]
  def apply[K, V](entries: (K, V)*): HashMap[K, V] = HashMap.from(entries)
  def from[K, V](entries: IterableOnce[(K, V)]): HashMap[K, V] = HashMap.from(entries)

// The hash map keeps insertion order, which is all a LinkedHashMap adds.
type LinkedHashMap[K, V] = HashMap[K, V]

// scala-library's map specialised to reference keys, which boopickle keeps its references in.
type AnyRefMap[K <: AnyRef, V] = HashMap[K, V]

object AnyRefMap:
  def empty[K <: AnyRef, V]: HashMap[K, V] = HashMap.empty[K, V]
  def apply[K <: AnyRef, V](entries: (K, V)*): HashMap[K, V] = HashMap.from(entries)
  def from[K <: AnyRef, V](entries: IterableOnce[(K, V)]): HashMap[K, V] = HashMap.from(entries)

object LinkedHashMap:
  def empty[K, V]: HashMap[K, V] = HashMap.empty[K, V]
  def apply[K, V](entries: (K, V)*): HashMap[K, V] = HashMap.from(entries)
  def from[K, V](entries: IterableOnce[(K, V)]): HashMap[K, V] = HashMap.from(entries)

final class HashSet[A](private val raw: RawMap[A, Boolean]) extends IterableOps[A, HashSet, HashSet[A]], Iterable[A], scala.collection.Set[A]:
  def this() = this(newRawMap[A, Boolean])
  def sizeHint(size: Int): Unit = ()
  // A mutable set is its own builder, as scala-library's `SetOps` is.
  def result(): HashSet[A] = this
  def buildCC[B](items: RawBuffer[B]): HashSet[B] = HashSet.from(Vector.wrap(items))
  @jvmEvidence
  @jvm("rt $0 $1 rtcall arrayOf(Lscala/IterableOnce;Lscala/reflect/ClassTag;)Ljava/lang/Object; cast_result")
  override def toArray[B >: A: scala.reflect.ClassTag]: Array[B] = unsafeCast(raw.rawKeys)
  override def size: Int = raw.rawSize
  override def knownSize: Int = raw.rawSize
  override def isEmpty: Boolean = raw.rawSize == 0
  def contains(elem: A): Boolean = raw.rawHas(elem)
  def apply(elem: A): Boolean = raw.rawHas(elem)
  def add(elem: A): Boolean =
    val isNew = !raw.rawHas(elem)
    raw.rawSet(elem, true)
    isNew
  def +=(elem: A): this.type =
    raw.rawSet(elem, true)
    this
  def -=(elem: A): this.type =
    raw.rawDelete(elem)
    this
  def addOne(elem: A): this.type = this += elem
  def subtractOne(elem: A): this.type = this -= elem
  def ++=(elems: IterableOnce[A]): this.type =
    elems.foreach(x => raw.rawSet(x, true))
    this
  def addAll(elems: IterableOnce[A]): this.type = this ++= elems
  def --=(elems: IterableOnce[A]): HashSet[A] =
    elems.foreach(x => raw.rawDelete(x))
    this
  def remove(elem: A): Boolean = raw.rawDelete(elem)
  def clear(): Unit = raw.rawClear()
  override def clone(): HashSet[A] = HashSet.from(this)
  def filterInPlace(p: A => Boolean): HashSet[A] =
    raw.rawKeys.foreach(x => if !p(x) then raw.rawDelete(x))
    this
  def foreach[U](f: A => U): Unit = raw.rawKeys.foreach(f)
  def equals(that: Any): Boolean = that match
    case s: scala.Set[?] => size == s.size && forall(x => s.contains(unsafeCast(x)))
    case s: HashSet[?] => size == s.size && forall(x => s.contains(unsafeCast(x)))
    case _ => false
  override def hashCode: Int = setHash(this)
  override def collectionClassName: String = "HashSet"
  override def toString: String = mkString("HashSet(", ", ", ")")

object HashSet extends IterableFactory[HashSet]:
  def empty[A]: HashSet[A] = new HashSet(newRawMap[A, Boolean])
  override def apply[A](elems: A*): HashSet[A] = from(elems)
  def from[A](elems: IterableOnce[A]): HashSet[A] = empty[A] ++= elems
  def newBuilder[A]: Builder[A, HashSet[A]] = ArrayBuffer.empty[A].mapResult(b => from(b))

type Set[A] = HashSet[A]

object Set:
  def empty[A]: HashSet[A] = HashSet.empty[A]
  def apply[A](elems: A*): HashSet[A] = HashSet.from(elems)
  def from[A](elems: IterableOnce[A]): HashSet[A] = HashSet.from(elems)

type LinkedHashSet[A] = HashSet[A]

/** `scala.collection.mutable.SortedSet` over a sorted array, as cats' `distinct` and the
  * application's sorted sets use it; every one is a `TreeSet`, the class keeping the two types
  * distinct as scala-library does. */
class SortedSet[A](private var items: RawBuffer[A], val ordering: Ordering[A]) extends IterableOps[A, Iterable, SortedSet[A]], Iterable[A]:
  def buildCC[B](elems: RawBuffer[B]): Iterable[B] = fromArray(elems)
  override def buildC(sorted: RawBuffer[A]): SortedSet[A] = SortedSet.from(Vector.wrap(sorted))(using ordering)
  override def iterator: Iterator[A] = Iterator.over(copyArray(items))
  @jvmEvidence
  @jvm("rt $0 $1 rtcall arrayOf(Lscala/IterableOnce;Lscala/reflect/ClassTag;)Ljava/lang/Object; cast_result")
  override def toArray[B >: A: scala.reflect.ClassTag]: Array[B] = unsafeCast(copyArray(items))
  override def size: Int = items.length
  override def knownSize: Int = items.length
  override def isEmpty: Boolean = items.length == 0
  def foreach[U](f: A => U): Unit = copyArray(items).foreach(f)
  def firstKey: A = head
  def lastKey: A = last
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
  def apply(elem: A): Boolean = contains(elem)
  def add(elem: A): Boolean =
    val i = search(elem)
    if i >= 0 then false
    else
      bufferInsert(items, -i - 1, elem)
      true
  def remove(elem: A): Boolean =
    val i = search(elem)
    if i < 0 then false
    else
      bufferRemove(items, i)
      true
  def +=(elem: A): this.type =
    add(elem)
    this
  def -=(elem: A): this.type =
    remove(elem)
    this
  def addOne(elem: A): this.type = this += elem
  def subtractOne(elem: A): this.type = this -= elem
  def ++=(elems: IterableOnce[A]): this.type =
    elems.foreach(x => add(x))
    this
  def addAll(elems: IterableOnce[A]): this.type = this ++= elems
  def --=(elems: IterableOnce[A]): this.type =
    elems.foreach(x => remove(x))
    this
  def subtractAll(elems: IterableOnce[A]): this.type = this --= elems
  def clear(): Unit = items = emptyBuffer[A]
  override def clone(): SortedSet[A] = SortedSet.from(this)(using ordering)
  def filterInPlace(p: A => Boolean): this.type =
    items = items.filter(p)
    this
  def rangeFrom(from: A): SortedSet[A] = filter(x => ordering.gteq(x, from))
  def rangeUntil(until: A): SortedSet[A] = filter(x => ordering.lt(x, until))
  def range(from: A, until: A): SortedSet[A] = filter(x => ordering.gteq(x, from) && ordering.lt(x, until))
  def minAfter(key: A): Option[A] = find(x => ordering.gteq(x, key))
  def maxBefore(key: A): Option[A] = lastOption(x => ordering.lt(x, key))
  private def lastOption(p: A => Boolean): Option[A] =
    var out: Option[A] = None
    items.foreach(x => if p(x) then out = Some(x))
    out
  override def map[B](f: A => B)(implicit ord: Ordering[B]): SortedSet[B] =
    val out = emptyBuffer[B]
    foreach(x => out.push(f(x)))
    SortedSet.sortedOrPlain(out, ord)
  override def flatMap[B](f: A => IterableOnce[B])(implicit ord: Ordering[B]): SortedSet[B] =
    val out = emptyBuffer[B]
    foreach(x => f(x).iterator.foreach(y => out.push(y)))
    SortedSet.sortedOrPlain(out, ord)
  def toSortedSet: scala.collection.immutable.SortedSet[A] = scala.collection.immutable.SortedSet.from(this)(using ordering)
  def equals(that: Any): Boolean = that match
    case s: SortedSet[?] => size == s.size && unsafeCast[SortedSet[?], SortedSet[A]](s).forall(x => contains(x))
    case s: scala.Set[?] => size == s.size && forall(x => unsafeCast[scala.Set[?], scala.Set[A]](s).contains(x))
    case _ => false
  override def hashCode: Int = setHash(this)
  override def collectionClassName: String = "TreeSet"
  override def toString: String = mkString("TreeSet(", ", ", ")")

object SortedSet:
  def sortedOrPlain[B](items: RawBuffer[B], ord: Ordering[B]): SortedSet[B] =
    if js.isUndefined(ord) then unsafeCast(HashSet.from(Vector.wrap(items))) else from(Vector.wrap(items))(using ord)
  def empty[A](implicit ord: Ordering[A]): SortedSet[A] = new TreeSet(emptyBuffer[A], ord)
  def apply[A](elems: A*)(implicit ord: Ordering[A]): SortedSet[A] = from(elems)
  def from[A](elems: IterableOnce[A])(implicit ord: Ordering[A]): SortedSet[A] =
    val out = new TreeSet(emptyBuffer[A], ord)
    elems.foreach(x => out.add(x))
    out
  def newBuilder[A](implicit ord: Ordering[A]): Builder[A, SortedSet[A]] = ArrayBuffer.empty[A].mapResult(items => from(items))

final class TreeSet[A](items: RawBuffer[A], ordering: Ordering[A]) extends SortedSet[A](items, ordering)

object TreeSet:
  def empty[A](implicit ord: Ordering[A]): TreeSet[A] = new TreeSet(emptyBuffer[A], ord)
  def apply[A](elems: A*)(implicit ord: Ordering[A]): TreeSet[A] = from(elems)
  def from[A](elems: IterableOnce[A])(implicit ord: Ordering[A]): TreeSet[A] = unsafeCast(SortedSet.from(elems))
  def newBuilder[A](implicit ord: Ordering[A]): Builder[A, TreeSet[A]] = ArrayBuffer.empty[A].mapResult(items => from(items))

object LinkedHashSet extends IterableFactory[HashSet]:
  def empty[A]: HashSet[A] = HashSet.empty[A]
  override def apply[A](elems: A*): HashSet[A] = HashSet.from(elems)
  def from[A](elems: IterableOnce[A]): HashSet[A] = HashSet.from(elems)
  def newBuilder[A]: Builder[A, HashSet[A]] = HashSet.newBuilder[A]

// A binary heap in an array whose slot 0 is unused, laid out as Scala's, so that iteration and
// printing give the same order.
final class PriorityQueue[A](implicit ord: Ordering[A]) extends IterableOps[A, Iterable, PriorityQueue[A]], Iterable[A]:
  private val heap: RawBuffer[A] = arrayOfOne(unsafeCast(()))
  def buildCC[B](items: RawBuffer[B]): Iterable[B] = fromArray(items)
  override def buildC(items: RawBuffer[A]): PriorityQueue[A] = PriorityQueue.from(Vector.wrap(items))
  def length: Int = heap.length - 1
  override def size: Int = length
  override def knownSize: Int = length
  override def isEmpty: Boolean = heap.length <= 1
  def foreach[U](f: A => U): Unit =
    var i = 1
    while i < heap.length do
      f(heap(i))
      i += 1
  override def iterator: Iterator[A] = Iterator.over(arraySlice(heap, 1, heap.length))
  @jvmEvidence
  @jvm("rt $0 $1 rtcall arrayOf(Lscala/IterableOnce;Lscala/reflect/ClassTag;)Ljava/lang/Object; cast_result")
  override def toArray[B >: A: scala.reflect.ClassTag]: Array[B] = unsafeCast(arraySlice(heap, 1, heap.length))
  private def swap(i: Int, j: Int): Unit =
    val h = heap(i)
    heap(i) = heap(j)
    heap(j) = h
  private def fixUp(m: Int): Unit =
    var k = m
    while k > 1 && ord.lt(heap(k / 2), heap(k)) do
      swap(k, k / 2)
      k = k / 2
  private def fixDown(m: Int, n: Int): Unit =
    var k = m
    var settled = false
    while !settled && n >= 2 * k do
      var j = 2 * k
      if j < n && ord.lt(heap(j), heap(j + 1)) then j += 1
      if ord.gteq(heap(k), heap(j)) then settled = true
      else
        swap(k, j)
        k = j
  def addOne(elem: A): this.type =
    heap.push(elem)
    fixUp(heap.length - 1)
    this
  def +=(elem: A): this.type = addOne(elem)
  def enqueue(elems: A*): Unit = elems.foreach(addOne)
  // Elements added to an empty queue are heapified in one pass, as in Scala.
  def addAll(elems: IterableOnce[A]): this.type =
    if heap.length > 1 then elems.foreach(addOne)
    else
      elems.foreach(x => heap.push(x))
      val n = length
      var i = n / 2
      while i >= 1 do
        fixDown(i, n)
        i -= 1
    this
  def ++=(elems: IterableOnce[A]): this.type = addAll(elems)
  def dequeue(): A =
    if isEmpty then noSuchElement("no element to remove from heap")
    else
      val result = heap(1)
      heap(1) = heap(heap.length - 1)
      bufferRemove(heap, heap.length - 1)
      fixDown(1, heap.length - 1)
      result
  def dequeueAll: Seq[A] =
    val out = emptyBuffer[A]
    while !isEmpty do out.push(dequeue())
    new ArraySeq(untaggedArray(out))
  override def head: A = if isEmpty then noSuchElement("queue is empty") else heap(1)
  override def headOption: Option[A] = if isEmpty then None else Some(heap(1))
  def clear(): Unit = bufferRemoveRange(heap, 1, heap.length - 1)
  override def clone(): PriorityQueue[A] = PriorityQueue.from(this)
  def reverse: PriorityQueue[A] = PriorityQueue.from(this)(using ord.reverse)
  override def collectionClassName: String = "PriorityQueue"
  override def toString: String = mkString("PriorityQueue(", ", ", ")")

object PriorityQueue:
  def empty[A](implicit ord: Ordering[A]): PriorityQueue[A] = new PriorityQueue[A]
  def apply[A](elems: A*)(implicit ord: Ordering[A]): PriorityQueue[A] = from(elems)
  def from[A](elems: IterableOnce[A])(implicit ord: Ordering[A]): PriorityQueue[A] = empty[A].addAll(elems)

/** scala-library's `ArrayBuilder`, over the array it builds: what `Array.newBuilder` gives. */
class ArrayBuilder[T] extends Builder[T, Array[T]]:
  private var items: RawBuffer[T] = emptyBuffer[T]
  def addOne(elem: T): this.type =
    items.push(elem)
    this
  override def +=(elem: T): this.type = addOne(elem)
  override def addAll(elems: IterableOnce[T]): this.type =
    elems.foreach(e => items.push(e))
    this
  def addAll(xs: Array[? <: T]): this.type = addAll(xs, 0, xs.length)
  def addAll(xs: Array[? <: T], offset: Int, length: Int): this.type =
    val elems: Array[T] = unsafeCast(xs)
    var i = offset
    while i < offset + length do
      items.push(elems(i))
      i += 1
    this
  override def ++=(elems: IterableOnce[T]): this.type = addAll(elems)
  def result(): Array[T] =
    val out = items
    items = emptyBuffer[T]
    builtArray(out, this)
  def clear(): Unit = items = emptyBuffer[T]
  def length: Int = items.length
  def knownSize: Int = items.length

object ArrayBuilder:
  @jvm("rt $1 rtcall taggedBuilder(Lscala/reflect/ClassTag;)Ljava/lang/Object; cast_result")
  def make[T](using ct: scala.reflect.ClassTag[T]): ArrayBuilder[T] = new ArrayBuilder[T]
  // scala-library's builders per primitive, which a library names (zio's `ChunkBuilder`).
  final class ofByte extends ArrayBuilder[Byte]
  final class ofShort extends ArrayBuilder[Short]
  final class ofChar extends ArrayBuilder[Char]
  final class ofInt extends ArrayBuilder[Int]
  final class ofLong extends ArrayBuilder[Long]
  final class ofFloat extends ArrayBuilder[Float]
  final class ofDouble extends ArrayBuilder[Double]
  final class ofBoolean extends ArrayBuilder[Boolean]
  final class ofUnit extends ArrayBuilder[Unit]
  final class ofRef[T <: AnyRef](using ct: scala.reflect.ClassTag[T]) extends ArrayBuilder[T]
