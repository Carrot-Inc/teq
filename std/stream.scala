package scala.collection.immutable

// scala-library's deprecated `Stream`, which libraries built on 2.13 still name (scalacheck shrinks with
// it): a list whose head is computed when its cell is made and whose tail on first use, remembered; `#::`
// takes its tail by name through `Stream.toDeferrer`, as scala-library's does.
@deprecated("Use LazyList instead of Stream", "2.13.0")
final class Stream[+A] private[immutable] (nonEmpty: Boolean, hd: A, tl: () => Stream[A]) extends scala.collection.AbstractSeq[A], LinearSeq[A], SeqOps[A, Stream, Stream[A]]:
  override def collectionClassName: String = "Stream"
  private var tailValue: Stream[A] = null
  private var tailForced = false
  override def isEmpty: Boolean = !nonEmpty
  override def head: A = if nonEmpty then hd else noSuchElement("head of empty stream")
  override def tail: Stream[A] =
    if !nonEmpty then unsupportedOperation("tail of empty stream")
    if !tailForced then
      tailValue = tl()
      tailForced = true
    tailValue
  def tailDefined: Boolean = tailForced
  override def headOption: Option[A] = if nonEmpty then Some(hd) else None
  def length: Int =
    var n = 0
    var cur: Stream[A] = this
    while !cur.isEmpty do
      n += 1
      cur = cur.tail
    n
  def apply(i: Int): A =
    var cur: Stream[A] = this
    var k = i
    while k > 0 && !cur.isEmpty do
      cur = cur.tail
      k -= 1
    if i < 0 || cur.isEmpty then throw new IndexOutOfBoundsException(i.toString) else cur.head
  def buildCC[B](items: RawBuffer[B]): Stream[B] = Stream.fromIterator(Iterator.over(items))
  override def iterator: Iterator[A] = new Stream.StreamIterator(this)
  def foreach[U](f: A => U): Unit =
    var cur: Stream[A] = this
    while !cur.isEmpty do
      f(cur.head)
      cur = cur.tail
  def force: Stream[A] =
    foreach(_ => ())
    this
  // Each operation makes its cells as they are reached, the head of each as its cell is made.
  private def lazily[B](f: Iterator[A] => Iterator[B]): Stream[B] = Stream.fromIterator(f(iterator))
  override def map[B](f: A => B): Stream[B] = if isEmpty then Stream.empty else Stream.cons(f(head), tail.map(f))
  override def flatMap[B](f: A => IterableOnce[B]): Stream[B] = lazily(_.flatMap(f))
  override def filter(p: A => Boolean): Stream[A] = lazily(_.filter(p))
  override def filterNot(p: A => Boolean): Stream[A] = lazily(_.filterNot(p))
  override def collect[B](pf: PartialFunction[A, B]): Stream[B] = lazily(_.collect(pf))
  // The last cell taken leaves the tail unforced, as scala-library's does.
  override def take(n: Int): Stream[A] =
    if n <= 0 || isEmpty then Stream.empty
    else if n == 1 then Stream.cons(head, Stream.empty)
    else Stream.cons(head, tail.take(n - 1))
  override def drop(n: Int): Stream[A] =
    var cur: Stream[A] = this
    var k = n
    while k > 0 && !cur.isEmpty do
      cur = cur.tail
      k -= 1
    cur
  override def dropWhile(p: A => Boolean): Stream[A] =
    var cur: Stream[A] = this
    while !cur.isEmpty && p(cur.head) do cur = cur.tail
    cur
  override def takeWhile(p: A => Boolean): Stream[A] = if isEmpty || !p(head) then Stream.empty else Stream.cons(head, tail.takeWhile(p))
  override def slice(from: Int, until: Int): Stream[A] = drop(from).take(until - math.max(from, 0))
  override def zip[B](that: IterableOnce[B]): Stream[(A, B)] = lazily(_.zip(that))
  override def zipWithIndex: Stream[(A, Int)] = lazily(_.zipWithIndex)
  override def scanLeft[B](z: B)(op: (B, A) => B): Stream[B] = lazily(_.scanLeft(z)(op))
  override def concat[B >: A](suffix: IterableOnce[B]): Stream[B] =
    if isEmpty then Stream.from(suffix) else Stream.cons(head, tail.concat(suffix))
  def append[B >: A](rest: => IterableOnce[B]): Stream[B] =
    if isEmpty then Stream.from(rest) else Stream.cons(head, tail.append(rest))
  override def tapEach[U](f: A => U): Stream[A] = lazily(_.tapEach(f))
  // The cells computed so far, as scala-library's `addStringNoForce` shows them.
  override def toString: String =
    val parts = emptyBuffer[String]
    var cur: Stream[A] = this
    var go = !cur.isEmpty
    while go do
      parts.push("" + cur.head)
      if !cur.tailForced then
        parts.push("<not computed>")
        go = false
      else
        cur = cur.tail
        go = !cur.isEmpty
    "Stream(" + parts.mkString(", ") + ")"

@deprecated("Use LazyList instead of Stream", "2.13.0")
object Stream extends SeqFactory[Stream]:
  // scala-library's `LinearSeqIterator`: the tail after the head `next` returns is a call-by-need
  // cell, forced by the next `hasNext` or `next`, never by the `next` that reads the head.
  private[immutable] final class StreamIterator[A](coll: Stream[A]) extends scala.collection.AbstractIterator[A]:
    private final class LazyCell(st: => Stream[A]):
      lazy val v = st
    private var these: LazyCell =
      val initialHead = coll
      new LazyCell(initialHead)
    def hasNext: Boolean = these.v.nonEmpty
    def next(): A =
      if isEmpty then Iterator.empty.next()
      else
        val cur = these.v
        val result = cur.head
        these = new LazyCell(cur.tail)
        result
  private val emptyStream: Stream[Nothing] = new Stream[Nothing](false, unsafeCast[Null, Nothing](null), () => null)
  def empty[A]: Stream[A] = emptyStream
  object cons:
    def apply[A](hd: A, tl: => Stream[A]): Stream[A] = new Stream(true, hd, () => tl)
  // The extractor `h #:: t`.
  object #:: :
    def unapply[A](s: Stream[A]): Option[(A, Stream[A])] = if s.isEmpty then None else Some((s.head, s.tail))
  /** What `x #:: s` and `p #::: s` take their stream `s` through, unevaluated. */
  final class Deferrer[A] private[Stream] (l: () => Stream[A]):
    def #::[B >: A](elem: => B): Stream[B] = cons(elem, l())
    def #:::[B >: A](prefix: Stream[B]): Stream[B] = prefix.append(l())
  implicit def toDeferrer[A](l: => Stream[A]): Deferrer[A] = new Deferrer(() => l)
  override def apply[A](elems: A*): Stream[A] = fromIterator(elems.iterator)
  def from[A](source: IterableOnce[A]): Stream[A] = source match
    case s: Stream[A @unchecked] => s
    case _ => fromIterator(source.iterator)
  def from(start: Int, step: Int): Stream[Int] = cons(start, from(start + step, step))
  def from(start: Int): Stream[Int] = from(start, 1)
  def newBuilder[A]: scala.collection.mutable.Builder[A, Stream[A]] =
    scala.collection.mutable.ArrayBuffer.empty[A].mapResult(items => fromIterator(items.iterator))
  def fromIterator[A](it: Iterator[A]): Stream[A] = if it.hasNext then cons(it.next(), fromIterator(it)) else empty
  def iterate[A](start: A)(f: A => A): Stream[A] = cons(start, iterate(f(start))(f))
  def continually[A](elem: => A): Stream[A] = cons(elem, continually(elem))
  override def fill[A](n: Int)(elem: => A): Stream[A] = if n <= 0 then empty else cons(elem, fill(n - 1)(elem))
  override def tabulate[A](n: Int)(f: Int => A): Stream[A] = iterate(0)(_ + 1).take(n).map(f)
  override def unfold[A, S](init: S)(f: S => Option[(A, S)]): Stream[A] = f(init) match
    case Some((a, s)) => cons(a, unfold(s)(f))
    case None => empty
