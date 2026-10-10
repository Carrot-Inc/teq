package scala

trait IterableOnce[+A]:
  def foreach[U](f: A => U): Unit
  def iterator: Iterator[A] = Iterator.over(iterableToArray(this))
  def knownSize: Int = -1
  def toMap[K, V](implicit ev: A <:< (K, V)): Map[K, V] = Map.from(unsafeCast[IterableOnce[A], IterableOnce[(K, V)]](this))
  def to[C1](factory: Factory[A, C1]): C1 = factory.fromSpecific(this)

// scala-library's deprecated extension methods of `IterableOnce`, which a library body compiled
// to build on both 2.12 and 2.13 calls (`IterableOnce.iterableOnceExtensionMethods(xs).foldLeft`).
object IterableOnce:
  implicit def iterableOnceExtensionMethods[A](it: IterableOnce[A]): IterableOnceExtensionMethods[A] = new IterableOnceExtensionMethods(it)

final class IterableOnceExtensionMethods[A](private val it: IterableOnce[A]):
  def foreach[U](f: A => U): Unit = it.foreach(f)
  def foldLeft[B](z: B)(op: (B, A) => B): B = it.iterator.foldLeft(z)(op)
  def foldRight[B](z: B)(op: (A, B) => B): B = it.iterator.toList.foldRight(z)(op)
  def reduceLeft[B >: A](op: (B, A) => B): B = it.iterator.reduceLeft(op)
  def size: Int = it.iterator.size
  def isEmpty: Boolean = !it.iterator.hasNext
  def nonEmpty: Boolean = it.iterator.hasNext
  def exists(p: A => Boolean): Boolean = it.iterator.exists(p)
  def forall(p: A => Boolean): Boolean = it.iterator.forall(p)
  def find(p: A => Boolean): Option[A] = it.iterator.find(p)
  def map[B](f: A => B): IterableOnce[B] = it.iterator.map(f)
  def flatMap[B](f: A => IterableOnce[B]): IterableOnce[B] = it.iterator.flatMap(f)
  def filter(p: A => Boolean): Iterator[A] = it.iterator.filter(p)
  def toList: List[A] = it.iterator.toList
  def toSeq: Seq[A] = it.iterator.toList
  def toVector: Vector[A] = it.iterator.toVector
  def toSet[B >: A]: Set[B] = it.iterator.toSet
  def mkString(start: String, sep: String, end: String): String = joinStrings(it, start, sep, end)
  def mkString(sep: String): String = joinStrings(it, "", sep, "")
  def mkString: String = joinStrings(it, "", "", "")

/** What `to(...)` builds a collection with: scala-library's `scala.collection.Factory`, which a
  * companion converts to (`xs.to(Map)`, `xs.to(Array)`).
  */
trait Factory[-A, +C]:
  def fromSpecific(it: IterableOnce[A]): C
  def newBuilder: scala.collection.mutable.Builder[A, C]

@js("$0.slice($1, $2)")
def arraySlice[A](arr: RawBuffer[A], from: Int, until: Int): RawBuffer[A] =
  val lo = scala.runtime.maxInt(from, 0)
  val hi = scala.runtime.minInt(until, arr.length)
  val out = emptyBuffer[A]
  var i = lo
  while i < hi do
    out.push(arr(i))
    i += 1
  out

@js("$0.reverse()")
@jvm("invokestatic java/util/Collections.reverse(Ljava/util/List;)V")
def reverseArray[A](arr: RawBuffer[A]): Unit

@js("[$0]")
def arrayOfOne[A](elem: A): RawBuffer[A] =
  val out = emptyBuffer[A]
  out.push(elem)
  out

def joinStrings(xs: IterableOnce[Any], start: String, sep: String, end: String): String =
  var s = start
  var first = true
  xs.foreach: x =>
    if first then first = false else s = s + sep
    s = s + x
  s + end

// Everything a collection offers beyond traversal. `CC` is the collection type that `map` and
// friends build, `C` the type of the collection itself; a class only has to provide `foreach`
// and `buildCC`. Traversals that may stop early go through `iterator`.
trait IterableOps[+A, +CC[_], +C] extends IterableOnce[A]:
  def buildCC[B](items: RawBuffer[B]): CC[B]
  def buildC(items: RawBuffer[A]): C = unsafeCast(buildCC(items))
  // The protocol of scala-library's `IterableOps`, which a library's collection class reaches
  // through the bodies of its own traits: the collection as `C`, a builder of it, its name.
  def coll: C = unsafeCast(this)
  def fromSpecific(it: IterableOnce[A]): C = buildC(iterableToArray(it))
  def newSpecificBuilder: scala.collection.mutable.Builder[A, C] =
    scala.collection.mutable.ArrayBuffer.empty[A].mapResult(items => buildC(rawItems(items)))
  def className: String = "Iterable"
  // The name scala-library's `toString` gives the collection, which a library reaches by
  // reflection (munit prints a collection so).
  def collectionClassName: String = className

  @jvmEvidence
  @jvm("rt $0 $1 rtcall arrayOf(Lscala/IterableOnce;Lscala/reflect/ClassTag;)Ljava/lang/Object; cast_result")
  def toArray[B >: A: scala.reflect.ClassTag]: Array[B] = unsafeCast(iterableToArray(this))
  def copyToArray[B >: A](dest: Array[B]): Int = copyToArray(dest, 0, Int.MaxValue)
  def copyToArray[B >: A](dest: Array[B], start: Int): Int = copyToArray(dest, start, Int.MaxValue)
  def copyToArray[B >: A](dest: Array[B], start: Int, len: Int): Int =
    val items = rawItems(this)
    var i = 0
    while i < items.length && i < len && start + i < dest.length do
      dest(start + i) = items(i)
      i += 1
    i
  def toList: List[A] = fromArray(rawItems(this))
  def toVector: Vector[A] = Vector.wrap(rawItems(this))
  def toSeq: Seq[A] = toList
  def toIterable: Iterable[A] = toList
  def toIterator: Iterator[A] = iterator
  def toIndexedSeq: IndexedSeq[A] = toVector
  def toSet[B >: A]: Set[B] = Set.from(this)
  def toBuffer[B >: A]: scala.collection.mutable.ArrayBuffer[B] = scala.collection.mutable.ArrayBuffer.from(this)
  def view: View[A] = new View(() => iterator, "View")

  def size: Int =
    var n = 0
    foreach(_ => n += 1)
    n
  def sizeIs: Int = size
  def sizeCompare(otherSize: Int): Int =
    if otherSize < 0 then 1
    else
      val known = knownSize
      if known >= 0 then java.lang.Integer.compare(known, otherSize)
      else
        var i = 0
        val it = iterator
        while i <= otherSize && it.hasNext do
          it.next()
          i += 1
        if i > otherSize then 1 else i - otherSize
  def sizeCompare(that: Iterable[?]): Int =
    val known = that.knownSize
    if known >= 0 then sizeCompare(known)
    else
      val mine = iterator
      val theirs = that.iterator
      while mine.hasNext && theirs.hasNext do
        mine.next()
        theirs.next()
      java.lang.Boolean.compare(mine.hasNext, theirs.hasNext)
  def isEmpty: Boolean = !iterator.hasNext
  def nonEmpty: Boolean = !isEmpty

  def head: A =
    val it = iterator
    if it.hasNext then it.next() else noSuchElement("next on empty iterator")
  def headOption: Option[A] =
    val it = iterator
    if it.hasNext then Some(it.next()) else None
  def last: A =
    val arr = rawItems(this)
    if arr.length == 0 then noSuchElement("next on empty iterator") else arr(arr.length - 1)
  def lastOption: Option[A] = if isEmpty then None else Some(last)
  // scala-library's message is none.
  def tail: C =
    if isEmpty then unsupportedOperation(null) else drop(1)
  def init: C =
    if isEmpty then unsupportedOperation(null) else dropRight(1)

  def exists(p: A => Boolean): Boolean =
    val it = iterator
    var found = false
    while !found && it.hasNext do found = p(it.next())
    found
  def forall(p: A => Boolean): Boolean = !exists(x => !p(x))
  def find(p: A => Boolean): Option[A] =
    val it = iterator
    var found: Option[A] = None
    while found.isEmpty && it.hasNext do
      val x = it.next()
      if p(x) then found = Some(x)
    found
  def count(p: A => Boolean): Int =
    var n = 0
    foreach(x => if p(x) then n += 1)
    n

  def foldLeft[B](z: B)(op: (B, A) => B): B =
    var acc = z
    foreach(x => acc = op(acc, x))
    acc
  def foldRight[B](z: B)(op: (A, B) => B): B =
    val arr = rawItems(this)
    var acc = z
    var i = arr.length - 1
    while i >= 0 do
      acc = op(arr(i), acc)
      i -= 1
    acc
  def fold[B >: A](z: B)(op: (B, B) => B): B = foldLeft(z)(op)
  def reduceOption[B >: A](op: (B, B) => B): Option[B] =
    var acc: Option[B] = None
    foreach: x =>
      acc = acc match
        case Some(a) => Some(op(a, x))
        case None => Some(x)
    acc
  def reduce[B >: A](op: (B, B) => B): B = reduceOption(op) match
    case Some(v) => v
    case None => unsupportedOperation("empty.reduceLeft")
  def reduceLeft[B >: A](op: (B, A) => B): B =
    if isEmpty then unsupportedOperation("empty.reduceLeft")
    else
      val it = iterator
      var acc: B = it.next()
      while it.hasNext do acc = op(acc, it.next())
      acc
  def reduceLeftOption[B >: A](op: (B, A) => B): Option[B] = if isEmpty then None else Some(reduceLeft(op))
  def reduceRight[B >: A](op: (A, B) => B): B =
    val arr = rawItems(this)
    if arr.length == 0 then unsupportedOperation("empty.reduceRight")
    else
      var acc: B = arr(arr.length - 1)
      var i = arr.length - 2
      while i >= 0 do
        acc = op(arr(i), acc)
        i -= 1
      acc
  def reduceRightOption[B >: A](op: (A, B) => B): Option[B] = if isEmpty then None else Some(reduceRight(op))

  def sum[B >: A](implicit num: Numeric[B]): B = foldLeft(num.zero)((a, b) => num.plus(a, b))
  def product[B >: A](implicit num: Numeric[B]): B = foldLeft(num.one)((a, b) => num.times(a, b))
  def maxOption[B >: A](implicit ord: Ordering[B]): Option[A] =
    reduceOption((a, b) => if ord.compare(a, b) >= 0 then a else b)
  def minOption[B >: A](implicit ord: Ordering[B]): Option[A] =
    reduceOption((a, b) => if ord.compare(a, b) <= 0 then a else b)
  def maxByOption[B](f: A => B)(implicit ord: Ordering[B]): Option[A] =
    reduceOption((a, b) => if ord.compare(f(a), f(b)) >= 0 then a else b)
  def minByOption[B](f: A => B)(implicit ord: Ordering[B]): Option[A] =
    reduceOption((a, b) => if ord.compare(f(a), f(b)) <= 0 then a else b)
  def max[B >: A](implicit ord: Ordering[B]): A = maxOption(using ord) match
    case Some(v) => v
    case None => unsupportedOperation("empty.max")
  def min[B >: A](implicit ord: Ordering[B]): A = minOption(using ord) match
    case Some(v) => v
    case None => unsupportedOperation("empty.min")
  def maxBy[B](f: A => B)(implicit ord: Ordering[B]): A = maxByOption(f) match
    case Some(v) => v
    case None => unsupportedOperation("empty.maxBy")
  def minBy[B](f: A => B)(implicit ord: Ordering[B]): A = minByOption(f) match
    case Some(v) => v
    case None => unsupportedOperation("empty.minBy")

  def mkString(start: String, sep: String, end: String): String = joinStrings(this, start, sep, end)
  def mkString(sep: String): String = mkString("", sep, "")
  def mkString: String = mkString("", "", "")

  def map[B](f: A => B): CC[B] =
    val out = emptyBuffer[B]
    foreach(x => out.push(f(x)))
    buildCC(out)
  def flatMap[B](f: A => IterableOnce[B]): CC[B] =
    val out = emptyBuffer[B]
    foreach(x => f(x).foreach(y => out.push(y)))
    buildCC(out)
  def flatten[B](implicit asIterable: A => IterableOnce[B]): CC[B] = flatMap(a => asIterable(a))
  def filter(p: A => Boolean): C =
    val out = emptyBuffer[A]
    foreach(x => if p(x) then out.push(x))
    buildC(out)
  def filterNot(p: A => Boolean): C = filter(x => !p(x))
  def withFilter(p: A => Boolean): C = filter(p)
  def collect[B](pf: PartialFunction[A, B]): CC[B] =
    val out = emptyBuffer[B]
    val miss = partialMiss[A, B]
    foreach: x =>
      val v = pf.applyOrElse(x, miss)
      if !isPartialMiss(v) then out.push(v)
    buildCC(out)
  def collectFirst[B](pf: PartialFunction[A, B]): Option[B] =
    val it = iterator
    val miss = partialMiss[A, B]
    var found: Option[B] = None
    while found.isEmpty && it.hasNext do
      val v = pf.applyOrElse(it.next(), miss)
      if !isPartialMiss(v) then found = Some(v)
    found
  def partition(p: A => Boolean): (C, C) =
    val yes = emptyBuffer[A]
    val no = emptyBuffer[A]
    foreach(x => if p(x) then yes.push(x) else no.push(x))
    (buildC(yes), buildC(no))
  def partitionMap[A1, A2](f: A => Either[A1, A2]): (CC[A1], CC[A2]) =
    val lefts = emptyBuffer[A1]
    val rights = emptyBuffer[A2]
    foreach: x =>
      f(x) match
        case Left(l) => lefts.push(l)
        case Right(r) => rights.push(r)
    (buildCC(lefts), buildCC(rights))
  def tapEach[U](f: A => U): C =
    foreach(f)
    buildC(rawItems(this))

  def slice(from: Int, until: Int): C =
    val out = emptyBuffer[A]
    val it = iterator
    var i = 0
    while i < until && it.hasNext do
      val x = it.next()
      if i >= from then out.push(x)
      i += 1
    buildC(out)
  def take(n: Int): C = slice(0, n)
  def drop(n: Int): C = slice(n, 2147483647)
  def takeRight(n: Int): C =
    val arr = rawItems(this)
    buildC(arraySlice(arr, Math.max(0, arr.length - Math.max(n, 0)), arr.length))
  def dropRight(n: Int): C =
    val arr = rawItems(this)
    buildC(arraySlice(arr, 0, Math.max(0, arr.length - Math.max(n, 0))))
  def splitAt(n: Int): (C, C) = (take(n), drop(n))
  def takeWhile(p: A => Boolean): C =
    val out = emptyBuffer[A]
    val it = iterator
    var go = true
    while go && it.hasNext do
      val x = it.next()
      if p(x) then out.push(x) else go = false
    buildC(out)
  def dropWhile(p: A => Boolean): C =
    val out = emptyBuffer[A]
    var dropping = true
    foreach: x =>
      if dropping && !p(x) then dropping = false
      if !dropping then out.push(x)
    buildC(out)
  def span(p: A => Boolean): (C, C) =
    val prefix = emptyBuffer[A]
    val rest = emptyBuffer[A]
    var taking = true
    foreach: x =>
      if taking && !p(x) then taking = false
      if taking then prefix.push(x) else rest.push(x)
    (buildC(prefix), buildC(rest))

  def concat[B >: A](suffix: IterableOnce[B]): CC[B] =
    val out: RawBuffer[B] = copyArray(rawItems(this))
    suffix.foreach(x => out.push(x))
    buildCC(out)
  def ++[B >: A](suffix: IterableOnce[B]): CC[B] = concat(suffix)

  def zip[B](that: IterableOnce[B]): CC[(A, B)] =
    val out = emptyBuffer[(A, B)]
    val a = iterator
    val b = that.iterator
    while a.hasNext && b.hasNext do out.push((a.next(), b.next()))
    buildCC(out)
  def zipWithIndex: CC[(A, Int)] =
    val out = emptyBuffer[(A, Int)]
    var i = 0
    foreach: x =>
      out.push((x, i))
      i += 1
    buildCC(out)
  def zipAll[A1 >: A, B](that: IterableOnce[B], thisElem: A1, thatElem: B): CC[(A1, B)] =
    val out = emptyBuffer[(A1, B)]
    val a = iterator
    val b = that.iterator
    while a.hasNext || b.hasNext do
      out.push((if a.hasNext then a.next() else thisElem, if b.hasNext then b.next() else thatElem))
    buildCC(out)
  def scanLeft[B](z: B)(op: (B, A) => B): CC[B] =
    val out = arrayOfOne(z)
    var acc = z
    foreach: x =>
      acc = op(acc, x)
      out.push(acc)
    buildCC(out)
  def scan[B >: A](z: B)(op: (B, B) => B): CC[B] = scanLeft(z)(op)

  def scanRight[B](z: B)(op: (A, B) => B): CC[B] =
    val arr = rawItems(this)
    val out = arrayOfOne(z)
    var acc = z
    var i = arr.length - 1
    while i >= 0 do
      acc = op(arr(i), acc)
      out.push(acc)
      i -= 1
    reverseArray(out)
    buildCC(out)

  def groupBy[K](f: A => K): Map[K, C] =
    val groups = newRawMap[K, RawBuffer[A]]
    foreach: x =>
      val k = f(x)
      if groups.rawHas(k) then groups.rawGet(k).push(x) else groups.rawSet(k, arrayOfOne(x))
    val out = newRawMap[K, Any]
    groups.rawKeys.foreach(k => out.rawSet(k, buildC(groups.rawGet(k))))
    Map.fromRaw(out)
  def groupMap[K, B](key: A => K)(f: A => B): Map[K, CC[B]] =
    val groups = newRawMap[K, RawBuffer[B]]
    foreach: x =>
      val k = key(x)
      if groups.rawHas(k) then groups.rawGet(k).push(f(x)) else groups.rawSet(k, arrayOfOne(f(x)))
    val out = newRawMap[K, Any]
    groups.rawKeys.foreach(k => out.rawSet(k, buildCC(groups.rawGet(k))))
    Map.fromRaw(out)
  def groupMapReduce[K, B](key: A => K)(f: A => B)(reduce: (B, B) => B): Map[K, B] =
    val out = newRawMap[K, Any]
    foreach: x =>
      val k = key(x)
      out.rawSet(k, if out.rawHas(k) then reduce(unsafeCast(out.rawGet(k)), f(x)) else f(x))
    Map.fromRaw(out)

  def sliding(size: Int, step: Int = 1): Iterator[C] = iterator.sliding(size, step).map(w => buildC(rawItems(w)))
  def grouped(size: Int): Iterator[C] =
    require(size > 0, s"size=$size and step=$size, but both must be positive")
    val arr = rawItems(this)
    val out = emptyBuffer[C]
    var i = 0
    while i < arr.length do
      out.push(buildC(arraySlice(arr, i, i + size)))
      i += size
    Iterator.over(out)

trait Iterable[+A] extends IterableOps[A, Iterable, Iterable[A]]

object Iterable:
  def apply[A](elems: A*): Iterable[A] = elems.toList
  def empty[A]: Iterable[A] = Nil
  def from[A](source: IterableOnce[A]): Iterable[A] = List.from(source)
  def fill[A](n: Int)(elem: => A): Iterable[A] = List.fill(n)(elem)

trait SeqOps[+A, +CC[_], +C] extends IterableOps[A, CC, C]:
  def length: Int
  def apply(i: Int): A
  override def className: String = "Seq"

  override def size: Int = length
  override def isEmpty: Boolean = length == 0
  override def toSeq: Seq[A] = unsafeCast(this)
  override def view: View[A] = new View(() => iterator, "SeqView")
  def lengthIs: Int = length
  def lengthCompare(len: Int): Int =
    val n = length
    if n < len then -1 else if n > len then 1 else 0
  def isDefinedAt(i: Int): Boolean = i >= 0 && i < length
  def lift(i: Int): Option[A] = if isDefinedAt(i) then Some(apply(i)) else None
  def indices: Range = Range(0, length, 1)

  def contains[B >: A](elem: B): Boolean = exists(x => x == elem)
  def indexWhere(p: A => Boolean, from: Int = 0): Int =
    val it = iterator
    var i = 0
    var at = -1
    while at < 0 && it.hasNext do
      val x = it.next()
      if i >= from && p(x) then at = i
      i += 1
    at
  def indexOf[B >: A](elem: B, from: Int = 0): Int = indexWhere(x => x == elem, from)
  def lastIndexWhere(p: A => Boolean, end: Int = Int.MaxValue): Int =
    val arr = rawItems(this)
    var i = if end < arr.length - 1 then end else arr.length - 1
    while i >= 0 && !p(arr(i)) do i -= 1
    i
  def lastIndexOf[B >: A](elem: B): Int = lastIndexWhere(x => x == elem)
  def findLast(p: A => Boolean): Option[A] =
    val arr = rawItems(this)
    val i = lastIndexWhere(p)
    if i < 0 then None else Some(arr(i))

  def reverse: C =
    val arr = rawItems(this)
    reverseArray(arr)
    buildC(arr)
  def reverseIterator: Iterator[A] =
    val arr = rawItems(this)
    reverseArray(arr)
    Iterator.over(arr)
  def sorted[B >: A](implicit ord: Ordering[B]): C =
    val arr = rawItems(this)
    sortArray(arr, (a, b) => ord.compare(a, b))
    buildC(arr)
  def sortBy[B](f: A => B)(implicit ord: Ordering[B]): C =
    val arr = rawItems(this)
    sortArray(arr, (a, b) => ord.compare(f(a), f(b)))
    buildC(arr)
  def sortWith(lt: (A, A) => Boolean): C =
    val arr = rawItems(this)
    sortArray(arr, (a, b) => if lt(a, b) then -1 else if lt(b, a) then 1 else 0)
    buildC(arr)
  def distinctBy[B](f: A => B): C =
    val seen = newRawMap[B, Boolean]
    val out = emptyBuffer[A]
    foreach: x =>
      val k = f(x)
      if !seen.rawHas(k) then
        seen.rawSet(k, true)
        out.push(x)
    buildC(out)
  def distinct: C = distinctBy(x => x)

  def updated[B >: A](index: Int, elem: B): CC[B] =
    val out: RawBuffer[B] = copyArray(rawItems(this))
    if index < 0 || index >= out.length then indexOutOfBounds(index.toString)
    out(index) = elem
    buildCC(out)
  def patch[B >: A](from: Int, other: IterableOnce[B], replaced: Int): CC[B] =
    val arr = rawItems(this)
    val start = Math.min(Math.max(from, 0), arr.length)
    val out: RawBuffer[B] = copyArray(arraySlice(arr, 0, start))
    other.foreach(x => out.push(x))
    var i = start + Math.max(replaced, 0)
    while i < arr.length do
      out.push(arr(i))
      i += 1
    buildCC(out)
  def appended[B >: A](elem: B): CC[B] =
    val out: RawBuffer[B] = copyArray(rawItems(this))
    out.push(elem)
    buildCC(out)
  def prepended[B >: A](elem: B): CC[B] =
    val out: RawBuffer[B] = arrayOfOne(elem)
    foreach(x => out.push(x))
    buildCC(out)
  def :+[B >: A](elem: B): CC[B] = appended(elem)
  def +:[B >: A](elem: B): CC[B] = prepended(elem)
  def appendedAll[B >: A](suffix: IterableOnce[B]): CC[B] = concat(suffix)
  def :++[B >: A](suffix: IterableOnce[B]): CC[B] = concat(suffix)
  def prependedAll[B >: A](prefix: IterableOnce[B]): CC[B] =
    val out: RawBuffer[B] = iterableToArray(prefix)
    foreach(x => out.push(x))
    buildCC(out)
  def ++:[B >: A](prefix: IterableOnce[B]): CC[B] = prependedAll(prefix)
  def padTo[B >: A](len: Int, elem: B): CC[B] =
    val out: RawBuffer[B] = copyArray(rawItems(this))
    while out.length < len do out.push(elem)
    buildCC(out)

  def tails: Iterator[C] =
    val arr = rawItems(this)
    Iterator.range(0, arr.length + 1).map(i => buildC(arraySlice(arr, i, arr.length)))
  def inits: Iterator[C] =
    val arr = rawItems(this)
    Iterator.range(0, arr.length + 1).map(i => buildC(arraySlice(arr, 0, arr.length - i)))

  def sameElements[B >: A](that: IterableOnce[B]): Boolean =
    val a = iterator
    val b = that.iterator
    var same = true
    while same && a.hasNext && b.hasNext do same = a.next() == b.next()
    same && !a.hasNext && !b.hasNext
  def corresponds[B](that: IterableOnce[B])(p: (A, B) => Boolean): Boolean =
    val a = iterator
    val b = that.iterator
    var same = true
    while same && a.hasNext && b.hasNext do same = p(a.next(), b.next())
    same && !a.hasNext && !b.hasNext
  def startsWith[B >: A](that: IterableOnce[B], offset: Int = 0): Boolean =
    val a = iterator.drop(offset)
    val b = that.iterator
    var same = true
    while same && b.hasNext do same = a.hasNext && a.next() == b.next()
    same
  def endsWith[B >: A](that: Iterable[B]): Boolean =
    val suffix = rawItems(that)
    val n = length
    n >= suffix.length && startsWith(Vector.wrap(suffix), n - suffix.length)
  def diff[B >: A](that: Seq[B]): C =
    val counts = newRawMap[B, Int]
    that.foreach(x => counts.rawSet(x, if counts.rawHas(x) then counts.rawGet(x) + 1 else 1))
    filter: x =>
      val n = if counts.rawHas(x) then counts.rawGet(x) else 0
      if n > 0 then counts.rawSet(x, n - 1)
      n == 0
  def intersect[B >: A](that: Seq[B]): C =
    val counts = newRawMap[B, Int]
    that.foreach(x => counts.rawSet(x, if counts.rawHas(x) then counts.rawGet(x) + 1 else 1))
    filter: x =>
      val n = if counts.rawHas(x) then counts.rawGet(x) else 0
      if n > 0 then counts.rawSet(x, n - 1)
      n > 0

@js("$seqHash($0)")
def seqHash(xs: IterableOnce[Any]): Int = scala.runtime.orderedHash(xs)

@js("$setHash($0)")
def setHash(xs: IterableOnce[Any]): Int = scala.runtime.unorderedHash(xs)

// Sequences of any kind are equal when their elements are, as in Scala. Both walk the elements
// in a loop: the equality of case classes would recurse once per element of a List.
/** `case head +: tail` on a sequence: scala-library's `scala.collection.+:`. */
object +: :
  def unapply[A, CC[_] <: Seq[?], C <: SeqOps[A, CC, C]](t: C & SeqOps[A, CC, C]): Option[(A, C)] =
    if t.isEmpty then None else Some((t.head, t.tail))

/** `case init :+ last` on a sequence: scala-library's `scala.collection.:+`. */
object :+ :
  def unapply[A, CC[_] <: Seq[?], C <: SeqOps[A, CC, C]](t: C & SeqOps[A, CC, C]): Option[(C, A)] =
    if t.isEmpty then None else Some((t.init, t.last))

trait Seq[+A] extends SeqOps[A, Seq, Seq[A]], Iterable[A]:
  def equals(that: Any): Boolean = that match
    case other: Seq[?] => sameElements(other)
    case _ => false
  override def hashCode: Int = seqHash(this)

object Seq:
  // scala-library's companion is an `IterableFactory`, whose `iterableFactory` a library body
  // names as the implicit `Factory` it resolved.
  implicit def iterableFactory[A]: Factory[A, Seq[A]] =
    new Factory[A, Seq[A]]:
      def fromSpecific(it: IterableOnce[A]): Seq[A] = from(it)
      def newBuilder: scala.collection.mutable.Builder[A, Seq[A]] = Seq.newBuilder[A]
  def apply[A](elems: A*): Seq[A] = elems.toList
  def empty[A]: Seq[A] = Nil
  def newBuilder[A]: scala.collection.mutable.Builder[A, Seq[A]] = List.newBuilder[A]
  def from[A](source: IterableOnce[A]): Seq[A] = List.from(source)
  def fill[A](n: Int)(elem: => A): Seq[A] = List.fill(n)(elem)
  def tabulate[A](n: Int)(f: Int => A): Seq[A] = List.tabulate(n)(f)
  def range(start: Int, end: Int): Seq[Int] = List.range(start, end)

// The traits a library's sequence class mixes in beside `IndexedSeq` (zio's `Chunk`): the
// operations are `SeqOps`'s, over `apply` and `length`.
trait IndexedSeqOps[+A, +CC[_], +C] extends SeqOps[A, CC, C]:
  override def className: String = "IndexedSeq"
trait StrictOptimizedIterableOps[+A, +CC[_], +C] extends IterableOps[A, CC, C]
trait StrictOptimizedSeqOps[+A, +CC[_], +C] extends SeqOps[A, CC, C], StrictOptimizedIterableOps[A, CC, C]
// A library's collection builds through its companion, as scala-library's `fromSpecific` does.
trait IterableFactoryDefaults[+A, +CC[_]] extends IterableOps[A, CC, CC[A]]:
  def iterableFactory: IterableFactory[CC]
  override def buildCC[B](items: RawBuffer[B]): CC[B] = iterableFactory.from(Vector.wrap(items))
  override def toString: String = mkString(className + "(", ", ", ")")

/** scala-library's `IterableFactory` and `SeqFactory`, which a library's companion extends. */
trait IterableFactory[+CC[_]]:
  def from[A](source: IterableOnce[A]): CC[A]
  def empty[A]: CC[A]
  def apply[A](elems: A*): CC[A] = from(elems)
  def newBuilder[A]: scala.collection.mutable.Builder[A, CC[A]]
  def fill[A](n: Int)(elem: => A): CC[A] = from(Vector.fill(n)(elem))
  def tabulate[A](n: Int)(f: Int => A): CC[A] = from(Vector.tabulate(n)(f))
  def iterate[A](start: A, len: Int)(f: A => A): CC[A] = from(Iterator.iterate(start)(f).take(len))
  def unfold[A, S](init: S)(f: S => Option[(A, S)]): CC[A] = from(Iterator.unfold(init)(f))
  def concat[A](xss: Iterable[A]*): CC[A] = from(xss.iterator.flatMap(_.iterator))
  def range[A](start: A, end: A)(using i: Integral[A]): CC[A] = from(NumericRange(start, end, i.one)(using i))
  def range[A](start: A, end: A, step: A)(using i: Integral[A]): CC[A] = from(NumericRange(start, end, step)(using i))
  implicit def iterableFactory[A]: Factory[A, CC[A]] = IterableFactory.toFactory(this)

object IterableFactory:
  implicit def toFactory[A, CC[_]](factory: IterableFactory[CC]): Factory[A, CC[A]] =
    new Factory[A, CC[A]]:
      def fromSpecific(it: IterableOnce[A]): CC[A] = factory.from(it)
      def newBuilder: scala.collection.mutable.Builder[A, CC[A]] = factory.newBuilder[A]

trait SeqFactory[+CC[_]] extends IterableFactory[CC]:
  def unapplySeq[A](x: CC[A] @scala.annotation.unchecked.uncheckedVariance): Some[CC[A]] = Some(x)
trait StrictOptimizedSeqFactory[+CC[_]] extends SeqFactory[CC]

// A library's class implements `apply` and `length`; the traversals follow from them.
trait IndexedSeq[+A] extends SeqOps[A, IndexedSeq, IndexedSeq[A]], Seq[A]:
  def foreach[U](f: A => U): Unit =
    var i = 0
    while i < length do
      f(apply(i))
      i += 1
  override def iterator: Iterator[A] = Iterator.tabulate(length)(apply)
  override def view: View[A] = new View(() => iterator, "IndexedSeqView")
  def buildCC[B](items: RawBuffer[B]): IndexedSeq[B] = Vector.wrap(items)

object IndexedSeq:
  def apply[A](elems: A*): IndexedSeq[A] = Vector.from(elems)
  def empty[A]: IndexedSeq[A] = Vector.empty[A]
  def from[A](source: IterableOnce[A]): IndexedSeq[A] = Vector.from(source)
  def fill[A](n: Int)(elem: => A): IndexedSeq[A] = Vector.fill(n)(elem)
  def tabulate[A](n: Int)(f: Int => A): IndexedSeq[A] = Vector.tabulate(n)(f)
  def range(start: Int, end: Int): IndexedSeq[Int] = Range(start, end, 1).toVector

// `hasNext` and `next()` are the core, which a library's iterator class implements; the std's
// own iterators are closures over the two (`FnIterator`). Transformations are lazy where
// Scala's are, so an endless iterator can be cut with take, takeWhile and the like; the rest of
// IterableOps consumes the iterator.
abstract class Iterator[+A] extends IterableOps[A, Iterator, Iterator[A]]:
  def hasNext: Boolean
  def next(): A
  def foreach[U](f: A => U): Unit = while hasNext do f(next())
  override def iterator: Iterator[A] = this
  def buildCC[B](items: RawBuffer[B]): Iterator[B] = Iterator.over(items)
  def length: Int = size
  def nextOption(): Option[A] = if hasNext then Some(next()) else None
  def distinctBy[B](f: A => B): Iterator[A] =
    val seen = scala.collection.mutable.HashSet.empty[B]
    filter(a => seen.add(f(a)))
  def distinct: Iterator[A] = distinctBy(a => a)
  def contains[B >: A](elem: B): Boolean = exists(x => x == elem)
  def indexWhere(p: A => Boolean, from: Int = 0): Int =
    var i = 0
    var at = -1
    while at < 0 && hasNext do
      if p(next()) && i >= from then at = i
      i += 1
    at
  def indexOf[B >: A](elem: B, from: Int = 0): Int = indexWhere(x => x == elem, from)

  override def map[B](f: A => B): Iterator[B] = new FnIterator(() => hasNext, () => f(next()))
  override def flatMap[B](f: A => IterableOnce[B]): Iterator[B] =
    var inner: Iterator[B] = Iterator.empty
    def fill(): Boolean =
      while !inner.hasNext && hasNext do inner = f(next()).iterator
      inner.hasNext
    new FnIterator(() => fill(), () => if fill() then inner.next() else Iterator.exhausted)
  override def filter(p: A => Boolean): Iterator[A] =
    var buffered = false
    var item: A = unsafeCast(())
    def fill(): Boolean =
      while !buffered && hasNext do
        item = next()
        buffered = p(item)
      buffered
    def step(): A =
      if !fill() then Iterator.exhausted
      else
        buffered = false
        item
    new FnIterator(() => fill(), () => step())
  override def withFilter(p: A => Boolean): Iterator[A] = filter(p)
  override def filterNot(p: A => Boolean): Iterator[A] = filter(x => !p(x))
  override def collect[B](pf: PartialFunction[A, B]): Iterator[B] =
    val miss = partialMiss[A, B]
    var buffered = false
    var item: B = unsafeCast(())
    def fill(): Boolean =
      while !buffered && hasNext do
        item = pf.applyOrElse(next(), miss)
        buffered = !isPartialMiss(item)
      buffered
    def step(): B =
      if !fill() then Iterator.exhausted
      else
        buffered = false
        item
    new FnIterator(() => fill(), () => step())
  override def takeWhile(p: A => Boolean): Iterator[A] =
    var open = true
    var buffered = false
    var item: A = unsafeCast(())
    def fill(): Boolean =
      if open && !buffered && hasNext then
        item = next()
        if p(item) then buffered = true else open = false
      buffered
    def step(): A =
      if !fill() then Iterator.exhausted
      else
        buffered = false
        item
    new FnIterator(() => fill(), () => step())
  override def dropWhile(p: A => Boolean): Iterator[A] =
    var dropping = true
    filter: x =>
      dropping = dropping && p(x)
      !dropping
  override def take(n: Int): Iterator[A] =
    var left = n
    def step(): A =
      left -= 1
      next()
    new FnIterator(() => left > 0 && hasNext, () => step())
  override def drop(n: Int): Iterator[A] =
    var left = n
    def skip(): Boolean =
      while left > 0 && hasNext do
        next()
        left -= 1
      hasNext
    def step(): A =
      skip()
      next()
    new FnIterator(() => skip(), () => step())
  override def slice(from: Int, until: Int): Iterator[A] = drop(from).take(Math.max(until - Math.max(from, 0), 0))
  override def zipWithIndex: Iterator[(A, Int)] =
    var i = -1
    map: x =>
      i += 1
      (x, i)
  override def zip[B](that: IterableOnce[B]): Iterator[(A, B)] =
    val other = that.iterator
    new FnIterator(() => hasNext && other.hasNext, () => (next(), other.next()))
  override def concat[B >: A](suffix: => IterableOnce[B]): Iterator[B] = this match
    case it: FnIterator[?] if it.appendSuffix.isDefined =>
      it.appendSuffix.get(() => suffix)
      unsafeCast(this)
    case _ =>
      val pending = emptyBuffer[() => IterableOnce[B]]
      pending.push(() => suffix)
      var current: Iterator[B] = this
      var taken = 0
      def fill(): Boolean =
        while !current.hasNext && taken < pending.length do
          current = pending(taken)().iterator
          taken += 1
        current.hasNext
      val it = new FnIterator[B](() => fill(), () => if fill() then current.next() else Iterator.exhausted)
      it.appendSuffix = Some(s => pending.push(unsafeCast(s)))
      it
  override def ++[B >: A](suffix: => IterableOnce[B]): Iterator[B] = concat(suffix)
  override def tapEach[U](f: A => U): Iterator[A] =
    map: x =>
      f(x)
      x
  override def scanLeft[B](z: B)(op: (B, A) => B): Iterator[B] =
    var acc = z
    var started = false
    def step(): B =
      if started then acc = op(acc, next()) else started = true
      acc
    new FnIterator(() => !started || hasNext, () => step())
  // Both halves pull from the source as needed; leading elements the trailing half runs past
  // wait in `lead`.
  override def span(p: A => Boolean): (Iterator[A], Iterator[A]) =
    val lead = emptyBuffer[A]
    var served = 0
    var leadEnded = false
    var boundary: Option[A] = None
    def pull(): Unit =
      if hasNext then
        val x = next()
        if p(x) then lead.push(x)
        else
          boundary = Some(x)
          leadEnded = true
      else leadEnded = true
    def leadHasNext(): Boolean =
      if served >= lead.length && !leadEnded then pull()
      served < lead.length
    def leadNext(): A =
      if leadHasNext() then
        served += 1
        lead(served - 1)
      else Iterator.exhausted
    def trailHasNext(): Boolean =
      while !leadEnded do pull()
      boundary.isDefined || hasNext
    def trailNext(): A =
      if trailHasNext() then
        boundary match
          case Some(x) =>
            boundary = None
            x
          case None => next()
      else Iterator.exhausted
    (new FnIterator(() => leadHasNext(), () => leadNext()), new FnIterator(() => trailHasNext(), () => trailNext()))
  override def grouped[B >: A](size: Int): GroupedIterator[B] = new GroupedIterator(this, size, size)
  override def sliding[B >: A](size: Int, step: Int = 1): GroupedIterator[B] = new GroupedIterator(this, size, step)
  override def toString: String = "<iterator>"

/** The windows of `grouped` and `sliding`: one starts every `step` elements, and a last one
  * shorter than `size` is given as it is, dropped after `withPartial(false)`, or filled up after
  * `withPadding(x)`; a window past the end is none. */
final class GroupedIterator[A](source: Iterator[A], size: Int, step: Int) extends Iterator[Seq[A]]:
  require(size > 0 && step > 0, s"size=$size and step=$step, but both must be positive")
  private val skipped = Math.max(step - size, 0)
  private var window = emptyBuffer[A]
  private var ready = false
  private var done = false
  private var partial = true
  private var pad: Option[() => A] = None
  def withPartial(x: Boolean): this.type =
    partial = x
    pad = None
    this
  def withPadding(x: => A): this.type =
    pad = Some(() => x)
    partial = true
    this
  private def fill(): Boolean =
    if !ready && !done then
      val first = window.length == 0
      val count = if first then size else step
      val dropped = if first then 0 else skipped
      val taken = emptyBuffer[A]
      while taken.length < count && source.hasNext do taken.push(source.next())
      if taken.length > dropped then
        window = arraySlice(window, Math.min(step, size), window.length)
        arraySlice(taken, dropped, taken.length).foreach(x => window.push(x))
        if window.length < size then
          pad match
            case Some(p) => while window.length < size do window.push(p())
            case None => if !partial then done = true
        ready = !done
      else done = true
    ready
  def hasNext: Boolean = fill()
  def next(): Seq[A] =
    if fill() then
      ready = false
      new ArraySeq(untaggedArray(copyArray(window)))
    else Iterator.exhausted

final class FnIterator[+A](more: () => Boolean, advance: () => A) extends Iterator[A]:
  // Set on the iterator that `concat` makes: a further ++ appends to its queue instead of nesting.
  private[scala] var appendSuffix: Option[(() => IterableOnce[Any]) => Unit] = None
  def hasNext: Boolean = more()
  def next(): A = advance()

object Iterator:
  def exhausted: Nothing = noSuchElement("next on empty iterator")
  def over[A](items: RawBuffer[A]): Iterator[A] = prefix(items, items.length)
  def prefix[A](items: RawBuffer[A], length: Int): Iterator[A] =
    var i = 0
    def step(): A =
      if i >= length then exhausted
      else
        i += 1
        items(i - 1)
    new FnIterator(() => i < length, () => step())
  def empty[A]: Iterator[A] = new FnIterator(() => false, () => exhausted)
  def apply[A](elems: A*): Iterator[A] = elems.iterator
  def single[A](elem: A): Iterator[A] = over(arrayOfOne(elem))
  def from(start: Int, step: Int = 1): Iterator[Int] = iterate(start)(x => x + step)
  def iterate[A](start: A)(f: A => A): Iterator[A] =
    var started = false
    var current = start
    def step(): A =
      if started then current = f(current) else started = true
      current
    new FnIterator(() => true, () => step())
  def continually[A](elem: => A): Iterator[A] = new FnIterator(() => true, () => elem)
  def fill[A](n: Int)(elem: => A): Iterator[A] = continually(elem).take(n)
  def tabulate[A](n: Int)(f: Int => A): Iterator[A] = from(0).take(n).map(f)
  def range(start: Int, end: Int, step: Int = 1): Iterator[Int] = Range(start, end, step).iterator
  def unfold[A, S](init: S)(f: S => Option[(A, S)]): Iterator[A] =
    var state = init
    var pending: Option[(A, S)] = None
    def fill(): Boolean =
      if pending.isEmpty then pending = f(state)
      pending.isDefined
    def step(): A = pending match
      case Some((a, s)) =>
        state = s
        pending = None
        a
      case None => if fill() then step() else exhausted
    new FnIterator(() => fill(), () => step())

// A collection traversed only on demand, anew each time, over the iterator of its source. The
// prefix names the kind of the source in toString; it survives the operations that keep a
// sequence a sequence, as in Scala.
final class View[+A](source: () => Iterator[A], prefix: String) extends SeqOps[A, View, View[A]], Iterable[A]:
  def buildCC[B](items: RawBuffer[B]): View[B] = new View(() => Iterator.over(items), prefix)
  private def lazily[B](f: Iterator[A] => Iterator[B]): View[B] = new View(() => f(source()), prefix)
  private def plainly[B](f: Iterator[A] => Iterator[B]): View[B] = new View(() => f(source()), "View")
  override def iterator: Iterator[A] = source()
  def foreach[U](f: A => U): Unit = source().foreach(f)
  override def view: View[A] = this
  override def isEmpty: Boolean = !source().hasNext
  def length: Int =
    var n = 0
    source().foreach(_ => n += 1)
    n
  def apply(i: Int): A =
    val it = source().drop(i)
    if i < 0 || !it.hasNext then indexOutOfBounds(i.toString) else it.next()
  override def toSeq: Seq[A] = toList
  def force: IndexedSeq[A] = toVector
  override def map[B](f: A => B): View[B] = lazily(_.map(f))
  override def take(n: Int): View[A] = lazily(_.take(n))
  override def drop(n: Int): View[A] = lazily(_.drop(n))
  override def slice(from: Int, until: Int): View[A] = lazily(_.slice(from, until))
  override def flatMap[B](f: A => IterableOnce[B]): View[B] = plainly(_.flatMap(f))
  override def filter(p: A => Boolean): View[A] = plainly(_.filter(p))
  override def collect[B](pf: PartialFunction[A, B]): View[B] = plainly(_.collect(pf))
  override def takeWhile(p: A => Boolean): View[A] = plainly(_.takeWhile(p))
  override def dropWhile(p: A => Boolean): View[A] = plainly(_.dropWhile(p))
  override def zip[B](that: IterableOnce[B]): View[(A, B)] = plainly(_.zip(that))
  override def zipWithIndex: View[(A, Int)] = plainly(_.zipWithIndex)
  override def scanLeft[B](z: B)(op: (B, A) => B): View[B] = plainly(_.scanLeft(z)(op))
  override def concat[B >: A](suffix: IterableOnce[B]): View[B] = plainly(_ ++ suffix)
  override def tapEach[U](f: A => U): View[A] = plainly(_.tapEach(f))
  override def collectionClassName: String = prefix
  override def toString: String = prefix + "(<not computed>)"

// A list whose cells are computed on first use and remembered.
final class LazyList[+A](compute: () => Option[(A, LazyList[A])]) extends IterableOps[A, LazyList, LazyList[A]], Iterable[A]:
  override def collectionClassName: String = "LazyList"
  private var forced = false
  private var cell: Option[(Any, LazyList[Any])] = None
  private def evaluated(): Option[(A, LazyList[A])] =
    if !forced then
      cell = unsafeCast(compute())
      forced = true
    unsafeCast(cell)
  def buildCC[B](items: RawBuffer[B]): LazyList[B] = LazyList.fromIterator(Iterator.over(items))
  override def isEmpty: Boolean = evaluated().isEmpty
  override def head: A = evaluated() match
    case Some((h, _)) => h
    case None => noSuchElement("head of empty lazy list")
  def tail: LazyList[A] = evaluated() match
    case Some((_, t)) => t
    case None => unsupportedOperation("tail of empty lazy list")
  override def headOption: Option[A] = evaluated().map(c => c._1)
  override def iterator: Iterator[A] =
    var cur: LazyList[A] = this
    def step(): A =
      val h = cur.head
      cur = cur.tail
      h
    new FnIterator(() => !cur.isEmpty, () => step())
  def foreach[U](f: A => U): Unit =
    var cur: LazyList[A] = this
    while !cur.isEmpty do
      f(cur.head)
      cur = cur.tail
  def #::[B >: A](elem: B): LazyList[B] = LazyList.cons(elem, this)
  def force: LazyList[A] =
    foreach(_ => ())
    this
  private def lazily[B](f: Iterator[A] => Iterator[B]): LazyList[B] = LazyList.fromIterator(f(iterator))
  override def map[B](f: A => B): LazyList[B] = new LazyList(() => evaluated().map((h, t) => (f(h), t.map(f))))
  override def flatMap[B](f: A => IterableOnce[B]): LazyList[B] = lazily(_.flatMap(f))
  override def filter(p: A => Boolean): LazyList[A] = lazily(_.filter(p))
  override def collect[B](pf: PartialFunction[A, B]): LazyList[B] = lazily(_.collect(pf))
  override def take(n: Int): LazyList[A] = new LazyList(() => if n <= 0 then None else evaluated().map((h, t) => (h, t.take(n - 1))))
  override def drop(n: Int): LazyList[A] = lazily(_.drop(n))
  override def slice(from: Int, until: Int): LazyList[A] = lazily(_.slice(from, until))
  override def takeWhile(p: A => Boolean): LazyList[A] = lazily(_.takeWhile(p))
  override def dropWhile(p: A => Boolean): LazyList[A] = lazily(_.dropWhile(p))
  override def zip[B](that: IterableOnce[B]): LazyList[(A, B)] = lazily(_.zip(that))
  override def zipWithIndex: LazyList[(A, Int)] = lazily(_.zipWithIndex)
  override def scanLeft[B](z: B)(op: (B, A) => B): LazyList[B] = lazily(_.scanLeft(z)(op))
  override def concat[B >: A](suffix: IterableOnce[B]): LazyList[B] =
    new LazyList(() => evaluated() match
      case Some((h, t)) => Some((h, t ++ suffix))
      case None => LazyList.fromIterator(suffix.iterator).evaluated())
  override def tapEach[U](f: A => U): LazyList[A] = lazily(_.tapEach(f))
  // Only the cells computed so far are shown.
  override def toString: String =
    val parts = emptyBuffer[String]
    var cur: LazyList[A] = this
    var go = true
    while go do
      if !cur.forced then
        parts.push("<not computed>")
        go = false
      else cur.evaluated() match
        case Some((h, t)) =>
          parts.push("" + h)
          cur = t
        case None => go = false
    "LazyList(" + parts.mkString(", ") + ")"

object LazyList:
  def empty[A]: LazyList[A] = new LazyList(() => None)
  object cons:
    def apply[A](head: => A, tail: => LazyList[A]): LazyList[A] = new LazyList(() => Some((head, tail)))
  def apply[A](elems: A*): LazyList[A] = fromIterator(elems.iterator)
  def from(start: Int, step: Int = 1): LazyList[Int] = iterate(start)(_ + step)
  def fromIterator[A](it: Iterator[A]): LazyList[A] =
    new LazyList(() => if it.hasNext then Some((it.next(), fromIterator(it))) else None)
  def iterate[A](start: A)(f: A => A): LazyList[A] = cons(start, iterate(f(start))(f))
  def continually[A](elem: => A): LazyList[A] = cons(elem, continually(elem))
  def fill[A](n: Int)(elem: => A): LazyList[A] = continually(elem).take(n)
  def tabulate[A](n: Int)(f: Int => A): LazyList[A] = iterate(0)(_ + 1).take(n).map(f)
  def range(start: Int, end: Int, step: Int = 1): LazyList[Int] = fromIterator(Range(start, end, step).iterator)
  def unfold[A, S](init: S)(f: S => Option[(A, S)]): LazyList[A] = new LazyList(() => f(init).map((a, s) => (a, unfold(s)(f))))

extension [A, CC[_]](xss: IterableOps[Iterable[A], CC, Any])
  def transpose: CC[CC[A]] =
    val rows = rawItems(xss).map(row => rawItems(row))
    val width = if rows.length == 0 then 0 else rows(0).length
    if rows.exists(row => row.length != width) then illegalArgument("transpose requires all collections have the same size")
    xss.buildCC(tabulatedBuffer(width)(i => xss.buildCC(rows.map(row => row(i)))))

extension [A, B, CC[_]](xs: IterableOps[(A, B), CC, Any])
  def unzip: (CC[A], CC[B]) = (xs.map(p => p._1), xs.map(p => p._2))

extension [A, B, C, CC[_]](xs: IterableOps[(A, B, C), CC, Any])
  def unzip3: (CC[A], CC[B], CC[C]) = (xs.map(p => p._1), xs.map(p => p._2), xs.map(p => p._3))
