package scala.collection

// The abstract classes a library's anonymous collection extends (cats' `toIterable`, scalactic's
// `prettyArray`), over the std's traits.
abstract class AbstractIterable[+A] extends Iterable[A]
abstract class AbstractIterator[+A] extends Iterator[A]
abstract class AbstractSeq[+A] extends Seq[A]

/** scala-library's `collection.Map`: what the immutable `Map` and the mutable `HashMap` share. */
trait Map[K, +V] extends Iterable[(K, V)]:
  def get(key: K): Option[V]
  // Of the map's kind, as scala-library's `MapOps` has them (scalactic's `GenMapDiffer` walks a
  // `collection.Map` by them); each map class has them as its own.
  def filter(p: ((K, V)) => Boolean): Map[K, V]
  def tail: Map[K, V]
  def contains(key: K): Boolean = get(key).isDefined
  def isDefinedAt(key: K): Boolean = contains(key)
  def apply(key: K): V = get(key) match
    case Some(v) => v
    case None => throw new NoSuchElementException("key not found: " + key)
  def getOrElse[V1 >: V](key: K, default: => V1): V1 = get(key) match
    case Some(v) => v
    case None => default

object Map:
  def empty[K, V]: scala.Map[K, V] = scala.Map.empty[K, V]
  def apply[K, V](entries: (K, V)*): scala.Map[K, V] = scala.Map.from(entries)
  def from[K, V](entries: IterableOnce[(K, V)]): scala.Map[K, V] = scala.Map.from(entries)

// scala-library's `collection.Set`, above the immutable `Set` and the mutable `HashSet`: what a
// library's signature says of a set it builds either way (izumi-reflect's `breakRefinement`
// gives a `mutable.LinkedHashSet` as one).
trait Set[A] extends Iterable[A]:
  def contains(elem: A): Boolean
  def apply(elem: A): Boolean = contains(elem)
  def subsetOf(that: Set[A]): Boolean = forall(that.contains)
  // `collection.SetOps`'s, deprecated since 2.13, which libraries built on it still call
  // (scalactic's `prettify`): the elements and the element, or the elements, an immutable set;
  // both alternatives, as scala-library has them, so that the immutable sets' overrides of the
  // two are named alike in the output.
  def +(elem: A): Set[A] = scala.Set.from(this) + elem
  def +(elem1: A, elem2: A, elems: A*): Set[A] = scala.Set.from(this).+(elem1, elem2, elems*)

object Set:
  def empty[A]: Set[A] = scala.Set.empty[A]
  def apply[A](elems: A*): Set[A] = scala.Set.from(elems)
  def from[A](elems: IterableOnce[A]): Set[A] = scala.Set.from(elems)

// scala-library's `BufferedIterator`: an iterator whose next element `head` reads without taking it.
trait BufferedIterator[+A] extends Iterator[A]:
  def head: A
  def headOption: Option[A] = if hasNext then Some(head) else None
  // scala-library's: an iterator already buffered is its own buffered iterator.
  override def buffered: this.type = this

// scala-library's value classes of `Predef`'s wrappers, whose operations teq's arrays and strings
// have without one: what a library's type test of a wrapper names (scalactic's
// `ArrayHelper.isArrayOps`, `ColCompatHelper.StringOps`), which no array or string passes, as
// none does under scalac.
final class ArrayOps[A](private val xs: Array[A]) extends AnyVal:
  def iterator: Iterator[A] = xs.iterator
  def size: Int = xs.length
  def length: Int = xs.length

final class StringOps(private val s: String) extends AnyVal:
  def mkString: String = s

/** scala-library's `LazyZip2`, what `xs.lazyZip(ys)` gives: the pairs of two collections, read off
  * their iterators as an operation asks for them (`LazyZipOps`). `exists` and `forall` stop at their
  * answer, and `map`, `flatMap` and `filter` hand `BuildFrom` a view that computes no element before
  * the builder reads it.
  */
final class LazyZip2[+El1, +El2, C1](src: C1, coll1: Iterable[El1], coll2: Iterable[El2]):
  // The pairs of the two collections' iterators, made afresh on each call.
  private def zipped[B](f: (El1, El2) => B): Iterator[B] =
    val elems1 = coll1.iterator
    val elems2 = coll2.iterator
    new scala.FnIterator(() => elems1.hasNext && elems2.hasNext, () => f(elems1.next(), elems2.next()))
  def map[B, C](f: (El1, El2) => B)(implicit bf: BuildFrom[C1, B, C]): C =
    bf.fromSpecific(src)(scala.View.over(() => zipped(f)))
  def flatMap[B, C](f: (El1, El2) => Iterable[B])(implicit bf: BuildFrom[C1, B, C]): C =
    bf.fromSpecific(src)(scala.View.over(() => {
      val elems1 = coll1.iterator
      val elems2 = coll2.iterator
      var current: Iterator[B] = Iterator.empty
      def advance(): Iterator[B] =
        while !current.hasNext && elems1.hasNext && elems2.hasNext do current = f(elems1.next(), elems2.next()).iterator
        current
      new scala.FnIterator(() => advance().hasNext, () => advance().next())
    }))
  def filter[C](p: (El1, El2) => Boolean)(implicit bf: BuildFrom[C1, (El1, El2), C]): C =
    bf.fromSpecific(src)(scala.View.over(() => {
      val elems1 = coll1.iterator
      val elems2 = coll2.iterator
      var current: Option[(El1, El2)] = None
      def advance(): Option[(El1, El2)] =
        while current.isEmpty && elems1.hasNext && elems2.hasNext do
          val e1 = elems1.next()
          val e2 = elems2.next()
          if p(e1, e2) then current = Some((e1, e2))
        current
      def take(): (El1, El2) = advance() match
        case Some(c) =>
          current = None
          c
        case None => Iterator.empty.next()
      new scala.FnIterator(() => advance().isDefined, () => take())
    }))
  def exists(p: (El1, El2) => Boolean): Boolean =
    val elems1 = coll1.iterator
    val elems2 = coll2.iterator
    var res = false
    while !res && elems1.hasNext && elems2.hasNext do res = p(elems1.next(), elems2.next())
    res
  def forall(p: (El1, El2) => Boolean): Boolean = !exists((el1, el2) => !p(el1, el2))
  def foreach[U](f: (El1, El2) => U): Unit =
    val elems1 = coll1.iterator
    val elems2 = coll2.iterator
    while elems1.hasNext && elems2.hasNext do f(elems1.next(), elems2.next())
  // What `lazyZip2ToIterable` gives: the pairs as a view.
  def unzip: (Iterable[El1], Iterable[El2]) = scala.View.over(() => zipped((a, b) => (a, b))).unzip
  override def toString: String = coll1.toString + ".lazyZip(" + coll2.toString + ")"

/** scala-library's `BuildFrom`: builds a collection like `from` with elements of type `A`, which
  * `ZIO.foreach` and `Future.traverse` take to return the collection they were given.
  */
trait BuildFrom[-From, -A, +C]:
  def fromSpecific(from: From)(it: IterableOnce[A]): C
  def newBuilder(from: From): scala.collection.mutable.Builder[A, C]
  def toFactory(from: From): Factory[A, C] = BuildFrom.factoryOf(this, from)

object BuildFrom extends BuildFromLowPriority1:
  def factoryOf[From, A, C](bf: BuildFrom[From, A, C], from: From): Factory[A, C] =
    new Factory[A, C]:
      def fromSpecific(it: IterableOnce[A]): C = bf.fromSpecific(from)(it)
      def newBuilder: scala.collection.mutable.Builder[A, C] = bf.newBuilder(from)

  implicit def buildFromMapOps[K0, V0, K, V]: BuildFrom[scala.Map[K0, V0], (K, V), scala.Map[K, V]] =
    new BuildFrom[scala.Map[K0, V0], (K, V), scala.Map[K, V]]:
      def fromSpecific(from: scala.Map[K0, V0])(it: IterableOnce[(K, V)]): scala.Map[K, V] = scala.Map.from(it)
      def newBuilder(from: scala.Map[K0, V0]): scala.collection.mutable.Builder[(K, V), scala.Map[K, V]] = scala.Map.newBuilder[K, V]

  implicit def buildFromSortedMapOps[K0, V0, K: Ordering, V]: BuildFrom[immutable.SortedMap[K0, V0], (K, V), immutable.SortedMap[K, V]] =
    new BuildFrom[immutable.SortedMap[K0, V0], (K, V), immutable.SortedMap[K, V]]:
      def fromSpecific(from: immutable.SortedMap[K0, V0])(it: IterableOnce[(K, V)]): immutable.SortedMap[K, V] = immutable.SortedMap.from(it)
      def newBuilder(from: immutable.SortedMap[K0, V0]): scala.collection.mutable.Builder[(K, V), immutable.SortedMap[K, V]] = immutable.SortedMap.newBuilder[K, V]

  implicit val buildFromString: BuildFrom[String, Char, String] =
    new BuildFrom[String, Char, String]:
      def fromSpecific(from: String)(it: IterableOnce[Char]): String = newBuilder(from).addAll(it).result()
      def newBuilder(from: String): scala.collection.mutable.Builder[Char, String] =
        scala.collection.mutable.ArrayBuffer.empty[Char].mapResult(_.mkString)

  implicit def buildFromArray[A: scala.reflect.ClassTag]: BuildFrom[Array[?], A, Array[A]] =
    new BuildFrom[Array[?], A, Array[A]]:
      def fromSpecific(from: Array[?])(it: IterableOnce[A]): Array[A] = taggedArray(iterableToArray(it))
      def newBuilder(from: Array[?]): scala.collection.mutable.Builder[A, Array[A]] =
        scala.collection.mutable.ArrayBuffer.empty[A].mapResult(_.toArray)

trait BuildFromLowPriority1 extends BuildFromLowPriority2:
  implicit def buildFromSortedSetOps[A0, A: Ordering]: BuildFrom[immutable.SortedSet[A0], A, immutable.SortedSet[A]] =
    new BuildFrom[immutable.SortedSet[A0], A, immutable.SortedSet[A]]:
      def fromSpecific(from: immutable.SortedSet[A0])(it: IterableOnce[A]): immutable.SortedSet[A] = immutable.SortedSet.from(it)
      def newBuilder(from: immutable.SortedSet[A0]): scala.collection.mutable.Builder[A, immutable.SortedSet[A]] = immutable.SortedSet.newBuilder[A]

  implicit def fallbackStringCanBuildFrom[A]: BuildFrom[String, A, IndexedSeq[A]] =
    new BuildFrom[String, A, IndexedSeq[A]]:
      def fromSpecific(from: String)(it: IterableOnce[A]): IndexedSeq[A] = IndexedSeq.from(it)
      def newBuilder(from: String): scala.collection.mutable.Builder[A, IndexedSeq[A]] =
        scala.collection.mutable.ArrayBuffer.empty[A].mapResult(items => IndexedSeq.from(items))

trait BuildFromLowPriority2:
  // A collection builds its own kind, which conforms to the static `CC` it was seen as: a `List`
  // seen as a `Seq` builds a `List`, a `Range` seen as an `IndexedSeq` builds an `IndexedSeq`.
  implicit def buildFromIterableOps[CC[X] <: Iterable[X], A0, A]: BuildFrom[CC[A0], A, CC[A]] =
    new BuildFrom[CC[A0], A, CC[A]]:
      def fromSpecific(from: CC[A0])(it: IterableOnce[A]): CC[A] =
        unsafeCast[CC[A0], IterableOps[A0, CC, ?]](from).buildCC(iterableToArray(it))
      def newBuilder(from: CC[A0]): scala.collection.mutable.Builder[A, CC[A]] =
        scala.collection.mutable.ArrayBuffer.empty[A].mapResult(items => fromSpecific(from)(items))

  implicit def buildFromIterator[A]: BuildFrom[Iterator[?], A, Iterator[A]] =
    new BuildFrom[Iterator[?], A, Iterator[A]]:
      def fromSpecific(from: Iterator[?])(it: IterableOnce[A]): Iterator[A] = it.iterator
      def newBuilder(from: Iterator[?]): scala.collection.mutable.Builder[A, Iterator[A]] =
        scala.collection.mutable.ArrayBuffer.empty[A].mapResult(_.iterator)

/** The companion of a collection built with evidence of its element type: scala-library's
  * `scala.collection.EvidenceIterableFactory` (`SortedSet` over an `Ordering`).
  */
trait EvidenceIterableFactory[+CC[_], Ev[_]]:
  def from[A](it: IterableOnce[A])(implicit ev: Ev[A]): CC[A]
  def empty[A](implicit ev: Ev[A]): CC[A]
  def newBuilder[A](implicit ev: Ev[A]): scala.collection.mutable.Builder[A, CC[A]]

object EvidenceIterableFactory:
  implicit def toFactory[Ev[_], A, CC[_]](factory: EvidenceIterableFactory[CC, Ev])(implicit ev: Ev[A]): Factory[A, CC[A]] =
    new Factory[A, CC[A]]:
      def fromSpecific(it: IterableOnce[A]): CC[A] = factory.from(it)
      def newBuilder: scala.collection.mutable.Builder[A, CC[A]] = factory.newBuilder[A]

type View[+A] = scala.View[A]

// scala-library's `SeqView` and `IndexedSeqView`, the views of a sequence and of an indexed one:
// an element is read by its index from the source (`apply` and `length` abstract, an indexed view's
// iterator reading them), each operation a lazy adapter over its source that keeps the kind of the
// view where scala-library's result type does (`map`, `take`, `reverse`); one that may not gives a
// `View` (`filter`, `flatMap`). A sorted view sorts its source once, at its first element asked.
trait SeqView[+A] extends SeqOps[A, View, View[A]], View[A]:
  override def view: SeqView[A] = this
  // A view is no sequence: its elements copied into one (`IterableOnceOps.toSeq`), as an adapter
  // extending a view class and `SeqView` would otherwise take the std's `SeqOps.toSeq`.
  override def toSeq: Seq[A] = toList
  override def map[B](f: A => B): SeqView[B] = new SeqView.Map(this, f)
  override def appended[B >: A](elem: B): SeqView[B] = new SeqView.Appended(this, elem)
  override def prepended[B >: A](elem: B): SeqView[B] = new SeqView.Prepended(elem, this)
  override def reverse: SeqView[A] = new SeqView.Reverse(this)
  override def take(n: Int): SeqView[A] = new SeqView.Take(this, n)
  override def drop(n: Int): SeqView[A] = new SeqView.Drop(this, n)
  override def takeRight(n: Int): SeqView[A] = new SeqView.TakeRight(this, n)
  override def dropRight(n: Int): SeqView[A] = new SeqView.DropRight(this, n)
  override def tapEach[U](f: A => U): SeqView[A] = new SeqView.Map(this, (a: A) => { f(a); a })
  // scala-library overloads these with a sequence, whose result stays a `SeqView`; the std hides
  // an inherited method of the name unless it declares the alternatives together.
  override def concat[B >: A](suffix: IterableOnce[B]): View[B] = super.concat(suffix)
  def concat[B >: A](suffix: SeqOps[B, AnyConstr, ?]): SeqView[B] = new SeqView.Concat(this, suffix)
  override def appendedAll[B >: A](suffix: IterableOnce[B]): View[B] = super.concat(suffix)
  def appendedAll[B >: A](suffix: SeqOps[B, AnyConstr, ?]): SeqView[B] = new SeqView.Concat(this, suffix)
  override def prependedAll[B >: A](prefix: IterableOnce[B]): View[B] = prefix match
    case xs: Iterable[?] => View.over(() => unsafeCast[Iterable[?], Iterable[B]](xs).iterator ++ iterator)
    case _ => View.from(prefix.iterator ++ iterator)
  def prependedAll[B >: A](prefix: SeqOps[B, AnyConstr, ?]): SeqView[B] = new SeqView.Concat(prefix, this)
  override def sorted[B >: A](implicit ord: Ordering[B]): SeqView[A] = new SeqView.Sorted(this, length, ord)
  // scala-library's `SeqOps.isEmpty` and `lengthCompare` (`IterableOps.sizeCompare`): a known size,
  // else the elements walked as far as the comparison needs; `size` is the length.
  override def isEmpty: Boolean = lengthCompare(0) == 0
  override def lengthCompare(len: Int): Int =
    if len < 0 then 1
    else
      val known = knownSize
      if known >= 0 then Integer.compare(known, len)
      else
        var i = 0
        val it = iterator
        var result = 0
        var done = false
        while !done && it.hasNext do
          if i == len then
            result = 1
            done = true
          else
            it.next()
            i += 1
        if done then result else i - len
  override def collectionClassName: String = "SeqView"

object SeqView:
  class Id[+A](underlying: SeqOps[A, AnyConstr, ?]) extends SeqView[A]:
    def apply(idx: Int): A = underlying.apply(idx)
    def length: Int = underlying.length
    def iterator: Iterator[A] = underlying.iterator
    override def knownSize: Int = underlying.knownSize
    override def isEmpty: Boolean = underlying.isEmpty
  // A linear sequence of the std that is no `SeqOps` (`LazyList`): indexed by walking it.
  final class OfLinear[+A](underlying: Iterable[A]) extends SeqView[A]:
    def apply(idx: Int): A =
      val it = underlying.iterator.drop(idx)
      if idx < 0 || !it.hasNext then indexOutOfBounds(idx.toString) else it.next()
    def length: Int = underlying.size
    def iterator: Iterator[A] = underlying.iterator
    override def isEmpty: Boolean = underlying.isEmpty
  class Map[+A, +B](underlying: SeqOps[A, AnyConstr, ?], f: A => B) extends scala.View.Map[A, B](underlying, f), SeqView[B]:
    def apply(idx: Int): B = f(underlying(idx))
    def length: Int = underlying.length
  class Appended[+A](underlying: SeqOps[A, AnyConstr, ?], elem: A) extends scala.View.Appended(underlying, elem), SeqView[A]:
    def apply(idx: Int): A = if idx == underlying.length then elem else underlying(idx)
    def length: Int = underlying.length + 1
  class Prepended[+A](elem: A, underlying: SeqOps[A, AnyConstr, ?]) extends scala.View.Prepended(elem, underlying), SeqView[A]:
    def apply(idx: Int): A = if idx == 0 then elem else underlying(idx - 1)
    def length: Int = underlying.length + 1
  class Concat[A](prefix: SeqOps[A, AnyConstr, ?], suffix: SeqOps[A, AnyConstr, ?]) extends scala.View.Concat[A](prefix, suffix), SeqView[A]:
    def apply(idx: Int): A =
      val l = prefix.length
      if idx < l then prefix(idx) else suffix(idx - l)
    def length: Int = prefix.length + suffix.length
  class Reverse[A](underlying: SeqOps[A, AnyConstr, ?]) extends SeqView[A]:
    def apply(i: Int): A = underlying.apply(size - 1 - i)
    def length: Int = underlying.size
    def iterator: Iterator[A] = underlying.reverseIterator
    override def knownSize: Int = underlying.knownSize
    override def isEmpty: Boolean = underlying.isEmpty
  class Take[+A](underlying: SeqOps[A, AnyConstr, ?], n: Int) extends scala.View.Take(underlying, n), SeqView[A]:
    def apply(idx: Int): A = if idx < n then underlying(idx) else indexOutOfBounds(idx.toString)
    def length: Int = Math.min(underlying.length, normN)
  class TakeRight[+A](underlying: SeqOps[A, AnyConstr, ?], n: Int) extends scala.View.TakeRight(underlying, n), SeqView[A]:
    private val delta = Math.max(underlying.size - Math.max(n, 0), 0)
    def length: Int = underlying.size - delta
    def apply(i: Int): A = underlying.apply(i + delta)
  class Drop[A](underlying: SeqOps[A, AnyConstr, ?], n: Int) extends scala.View.Drop[A](underlying, n), SeqView[A]:
    def length: Int = Math.max(underlying.size - normN, 0)
    def apply(i: Int): A = underlying.apply(i + normN)
    override def drop(n: Int): SeqView[A] = new Drop(underlying, this.n + n)
  class DropRight[A](underlying: SeqOps[A, AnyConstr, ?], n: Int) extends scala.View.DropRight[A](underlying, n), SeqView[A]:
    private val len = Math.max(underlying.size - Math.max(n, 0), 0)
    def length: Int = len
    def apply(i: Int): A = underlying.apply(i)
  // scala-library's `Sorted`: the source's length taken at once, its elements sorted once, when one
  // is first asked for. Sorted again by its order's reverse it is the same elements reversed
  // (`ReverseSorted`), by another order the source while it is unsorted (`elems`): neither sorts.
  class Sorted[A, B >: A](underlying0: SeqOps[A, AnyConstr, ?], len: Int, ord: Ordering[B]) extends SeqView[A]:
    outer =>
    private var underlying: SeqOps[A, AnyConstr, ?] = underlying0
    private var evaluated = false
    private lazy val sortedElems: Seq[A] =
      val res: Seq[A] =
        if len == 0 then Nil
        else if len == 1 then List(underlying.head)
        else underlying.toList.sorted(unsafeCast[Ordering[B], Ordering[A]](ord))
      evaluated = true
      underlying = null
      res
    private def elems: SeqOps[A, AnyConstr, ?] = if evaluated then sortedElems else underlying
    def apply(i: Int): A = sortedElems.apply(i)
    def length: Int = len
    def iterator: Iterator[A] = Iterator.empty ++ sortedElems.iterator
    override def knownSize: Int = len
    override def isEmpty: Boolean = len == 0
    override def reverse: SeqView[A] = new ReverseSorted
    override def sorted[B1 >: A](implicit ord1: Ordering[B1]): SeqView[A] =
      if ord1 == ord then this
      else if ord1.isReverseOf(ord) then reverse
      else new Sorted(elems, len, ord1)
    private class ReverseSorted extends SeqView[A]:
      private lazy val reversedElems = new SeqView.Reverse(sortedElems)
      def apply(i: Int): A = reversedElems.apply(i)
      def length: Int = len
      def iterator: Iterator[A] = Iterator.empty ++ reversedElems.iterator
      override def knownSize: Int = len
      override def isEmpty: Boolean = len == 0
      override def reverse: SeqView[A] = outer
      override def sorted[B1 >: A](implicit ord1: Ordering[B1]): SeqView[A] =
        if ord1 == ord then outer
        else if ord1.isReverseOf(ord) then this
        else new Sorted(elems, len, ord1)

trait IndexedSeqView[+A] extends IndexedSeqOps[A, View, View[A]], SeqView[A]:
  override def view: IndexedSeqView[A] = this
  // scala-library's `IndexedSeqOps.knownSize` and `lengthCompare`, which its `SeqView` leaves to
  // `IndexedSeqOps`: the size is the length, compared without reading an element.
  override def knownSize: Int = length
  override def lengthCompare(len: Int): Int = Integer.compare(length, len)
  def view(from: Int, until: Int): IndexedSeqView[A] = slice(from, until)
  override def iterator: Iterator[A] = new IndexedSeqView.IndexedSeqViewIterator(this)
  override def reverseIterator: Iterator[A] = new IndexedSeqView.IndexedSeqViewReverseIterator(this)
  override def appended[B >: A](elem: B): IndexedSeqView[B] = new IndexedSeqView.Appended(this, elem)
  override def prepended[B >: A](elem: B): IndexedSeqView[B] = new IndexedSeqView.Prepended(elem, this)
  override def take(n: Int): IndexedSeqView[A] = new IndexedSeqView.Take(this, n)
  override def takeRight(n: Int): IndexedSeqView[A] = new IndexedSeqView.TakeRight(this, n)
  override def drop(n: Int): IndexedSeqView[A] = new IndexedSeqView.Drop(this, n)
  override def dropRight(n: Int): IndexedSeqView[A] = new IndexedSeqView.DropRight(this, n)
  override def map[B](f: A => B): IndexedSeqView[B] = new IndexedSeqView.Map(this, f)
  override def reverse: IndexedSeqView[A] = new IndexedSeqView.Reverse(this)
  override def slice(from: Int, until: Int): IndexedSeqView[A] = new IndexedSeqView.Slice(this, from, until)
  override def tapEach[U](f: A => U): IndexedSeqView[A] = new IndexedSeqView.Map(this, (a: A) => { f(a); a })
  def concat[B >: A](suffix: SomeIndexedSeqOps[B]): IndexedSeqView[B] = new IndexedSeqView.Concat(this, unsafeCast[SomeIndexedSeqOps[B], SeqOps[B, AnyConstr, ?]](suffix))
  def appendedAll[B >: A](suffix: SomeIndexedSeqOps[B]): IndexedSeqView[B] = new IndexedSeqView.Concat(this, unsafeCast[SomeIndexedSeqOps[B], SeqOps[B, AnyConstr, ?]](suffix))
  def prependedAll[B >: A](prefix: SomeIndexedSeqOps[B]): IndexedSeqView[B] = new IndexedSeqView.Concat(unsafeCast[SomeIndexedSeqOps[B], SeqOps[B, AnyConstr, ?]](prefix), this)
  override def collectionClassName: String = "IndexedSeqView"

object IndexedSeqView:
  // scala-library's iterators of an indexed view, reading its length once and an element by index.
  final class IndexedSeqViewIterator[A](self: IndexedSeqView[A]) extends Iterator[A]:
    private var current = 0
    private var remainder = self.length
    override def knownSize: Int = remainder
    def hasNext: Boolean = remainder > 0
    def next(): A =
      if remainder > 0 then
        val r = self(current)
        current += 1
        remainder -= 1
        r
      else Iterator.empty.next()
    override def drop(n: Int): Iterator[A] =
      if n > 0 then
        current += n
        remainder = Math.max(0, remainder - n)
      this
  final class IndexedSeqViewReverseIterator[A](self: IndexedSeqView[A]) extends Iterator[A]:
    private var remainder = self.length
    private var pos = remainder - 1
    def hasNext: Boolean = remainder > 0
    def next(): A =
      if remainder > 0 then
        val r = self(pos)
        pos -= 1
        remainder -= 1
        r
      else Iterator.empty.next()
  class Id[+A](underlying: SeqOps[A, AnyConstr, ?]) extends SeqView.Id(underlying), IndexedSeqView[A]
  class Appended[+A](underlying: SeqOps[A, AnyConstr, ?], elem: A) extends SeqView.Appended(underlying, elem), IndexedSeqView[A]
  class Prepended[+A](elem: A, underlying: SeqOps[A, AnyConstr, ?]) extends SeqView.Prepended(elem, underlying), IndexedSeqView[A]
  class Concat[A](prefix: SeqOps[A, AnyConstr, ?], suffix: SeqOps[A, AnyConstr, ?]) extends SeqView.Concat[A](prefix, suffix), IndexedSeqView[A]
  class Take[A](underlying: SeqOps[A, AnyConstr, ?], n: Int) extends SeqView.Take(underlying, n), IndexedSeqView[A]
  class TakeRight[A](underlying: SeqOps[A, AnyConstr, ?], n: Int) extends SeqView.TakeRight(underlying, n), IndexedSeqView[A]
  class Drop[A](underlying: SeqOps[A, AnyConstr, ?], n: Int) extends SeqView.Drop[A](underlying, n), IndexedSeqView[A]
  class DropRight[A](underlying: SeqOps[A, AnyConstr, ?], n: Int) extends SeqView.DropRight[A](underlying, n), IndexedSeqView[A]
  class Map[A, B](underlying: SeqOps[A, AnyConstr, ?], f: A => B) extends SeqView.Map(underlying, f), IndexedSeqView[B]
  class Reverse[A](underlying: SeqOps[A, AnyConstr, ?]) extends SeqView.Reverse[A](underlying), IndexedSeqView[A]:
    override def reverse: IndexedSeqView[A] = underlying match
      case x: IndexedSeqView[?] => unsafeCast(x)
      case _ => super.reverse
  class Slice[A](underlying: SeqOps[A, AnyConstr, ?], from: Int, until: Int) extends IndexedSeqView[A]:
    protected val lo: Int = Math.max(from, 0)
    protected val hi: Int = Math.min(Math.max(until, 0), underlying.length)
    protected val len: Int = Math.max(hi - lo, 0)
    def apply(i: Int): A = underlying(lo + i)
    def length: Int = len

// scala-library's `ArrayOps.ArrayView`: an array's view, indexed as the array is, printed with
// its elements. It stands beside the view it extends, so that the file of the array's operations
// (collections.scala), whose classes every program that converts or extends an array completes,
// names the view only in the result of `view`, and the views' files are read when a program asks.
final class ArrayView[A](a: Array[A]) extends IndexedSeqView[A]:
  def length: Int = a.length
  def apply(i: Int): A = a(i)
  override def toString: String = mkString("ArrayView(", ", ", ")")

// scala-library's `collection.AnyConstr`, the type constructor of any sequence's operations.
type AnyConstr[X] = Any
// scala-library's `IndexedSeqView.SomeIndexedSeqOps`: the std's `IndexedSeq` and the library classes
// beside it are no `IndexedSeqOps`.
type SomeIndexedSeqOps[A] = IndexedSeqOps[A, AnyConstr, ?] | IndexedSeq[A]

// scala-library's deprecated names of the collections before 2.13 (scala/collection/package.scala),
// which libraries built on it still name.
type Traversable[+X] = Iterable[X]
type TraversableOnce[+X] = IterableOnce[X]
type GenTraversableOnce[+X] = IterableOnce[X]
type GenTraversable[+X] = Iterable[X]
type GenIterable[+X] = Iterable[X]
type GenSeq[+X] = Seq[X]
type GenSet[X] = Set[X]
type GenMap[K, +V] = Map[K, V]
