package scala

// scala-library's `View`: a collection traversed only on demand, anew each time, over the
// iterator of its source. A sequence's view is a `SeqView` and an indexed sequence's an
// `IndexedSeqView` (`scala.collection`), whose operations keep the kind of the view where
// scala-library's result types do; the ones here give a `View`.
trait View[+A] extends IterableOps[A, View, View[A]], Iterable[A]:
  def buildCC[B](items: RawBuffer[B]): View[B] = View.over(() => Iterator.over(items))
  def foreach[U](f: A => U): Unit = iterator.foreach(f)
  override def view: View[A] = this
  override def toSeq: Seq[A] = toList
  def force: IndexedSeq[A] = toVector
  // scala-library's adapters, each over its source and taking its elements anew at each traversal.
  override def map[B](f: A => B): View[B] = new View.Map(this, f)
  override def take(n: Int): View[A] = new View.Take(this, n)
  override def drop(n: Int): View[A] = new View.Drop(this, n)
  override def takeRight(n: Int): View[A] = new View.TakeRight(this, n)
  override def dropRight(n: Int): View[A] = new View.DropRight(this, n)
  override def slice(from: Int, until: Int): View[A] = new View.Drop(new View.Take(this, until), from)
  // scala-library's `tail` and `init`, a view dropping one element, and `tails` and `inits`, the
  // views taken on demand until one is empty (`IterableOps.iterateUntilEmpty`).
  override def tail: View[A] =
    if isEmpty then throw new UnsupportedOperationException
    drop(1)
  override def init: View[A] =
    if isEmpty then throw new UnsupportedOperationException
    dropRight(1)
  override def tails: Iterator[View[A]] = iterateUntilEmpty(_.tail)
  override def inits: Iterator[View[A]] = iterateUntilEmpty(_.init)
  private def iterateUntilEmpty(f: View[A] => View[A]): Iterator[View[A]] =
    Iterator.iterate(this: View[A])(f).takeWhile(_.nonEmpty) ++ Iterator.single(View.empty[A])
  override def flatMap[B](f: A => IterableOnce[B]): View[B] = View.over(() => iterator.flatMap(f))
  override def filter(p: A => Boolean): View[A] = View.over(() => iterator.filter(p))
  override def collect[B](pf: PartialFunction[A, B]): View[B] = View.over(() => iterator.collect(pf))
  override def takeWhile(p: A => Boolean): View[A] = View.over(() => iterator.takeWhile(p))
  override def dropWhile(p: A => Boolean): View[A] = View.over(() => iterator.dropWhile(p))
  override def zip[B](that: IterableOnce[B]): View[(A, B)] = View.over(() => iterator.zip(that))
  override def zipWithIndex: View[(A, Int)] = View.over(() => iterator.zipWithIndex)
  override def scanLeft[B](z: B)(op: (B, A) => B): View[B] = View.over(() => iterator.scanLeft(z)(op))
  // scala-library's `IterableOps.concat`: a collection's elements are taken on demand, an
  // iterator's at once into a `LazyList`, whose view is a `SeqView`.
  override def concat[B >: A](suffix: IterableOnce[B]): View[B] = suffix match
    case xs: Iterable[?] => View.over(() => iterator ++ unsafeCast[Iterable[?], Iterable[B]](xs))
    case _ => View.from(iterator ++ suffix.iterator)
  override def ++:[B >: A](prefix: IterableOnce[B]): View[B] = prefix match
    case xs: Iterable[?] => View.over(() => unsafeCast[Iterable[?], Iterable[B]](xs).iterator ++ iterator)
    case _ => View.from(prefix.iterator ++ iterator)
  override def tapEach[U](f: A => U): View[A] = new View.Map(this, (a: A) => { f(a); a })
  override def collectionClassName: String = "View"
  override def toString: String = collectionClassName + "(<not computed>)"

object View:
  def apply[A](elems: A*): View[A] = from(elems)
  def empty[A]: View[A] = over(() => Iterator.empty)
  def from[A](source: IterableOnce[A]): View[A] = source match
    case v: View[?] => unsafeCast(v)
    case it: Iterable[?] => over(() => unsafeCast(it.iterator))
    case _ => LazyList.fromIterator(source.iterator).view
  def over[A](source: () => Iterator[A]): View[A] = new Over(source)
  final class Over[+A](source: () => Iterator[A]) extends View[A]:
    def iterator: Iterator[A] = source()

  // scala-library's adapters of `View`, which those of `SeqView` extend.
  class Map[+A, +B](underlying: IterableOnce[A], f: A => B) extends View[B]:
    def iterator: Iterator[B] = underlying.iterator.map(f)
    override def knownSize: Int = underlying.knownSize
  class Appended[+A](underlying: IterableOnce[A], elem: A) extends View[A]:
    def iterator: Iterator[A] = underlying.iterator ++ Iterator.single(elem)
  class Prepended[+A](elem: A, underlying: IterableOnce[A]) extends View[A]:
    def iterator: Iterator[A] = Iterator.single(elem) ++ underlying.iterator
  class Concat[A](prefix: IterableOnce[A], suffix: IterableOnce[A]) extends View[A]:
    def iterator: Iterator[A] = prefix.iterator ++ suffix.iterator
  class Take[+A](underlying: IterableOnce[A], n: Int) extends View[A]:
    protected val normN: Int = Math.max(n, 0)
    def iterator: Iterator[A] = underlying.iterator.take(n)
  class Drop[A](underlying: IterableOnce[A], n: Int) extends View[A]:
    protected val normN: Int = Math.max(n, 0)
    def iterator: Iterator[A] = underlying.iterator.drop(n)
  class TakeRight[+A](underlying: IterableOnce[A], n: Int) extends View[A]:
    protected val normN: Int = Math.max(n, 0)
    def iterator: Iterator[A] = View.takeRightIterator(underlying.iterator, n)
  class DropRight[A](underlying: IterableOnce[A], n: Int) extends View[A]:
    protected val normN: Int = Math.max(n, 0)
    def iterator: Iterator[A] = View.dropRightIterator(underlying.iterator, n)

  // scala-library's iterators of the last `n` elements and of all but them: the first reads its
  // source whole when first asked, the second keeps `n` elements ahead of the one it gives.
  private[scala] def takeRightIterator[A](it: Iterator[A], n: Int): Iterator[A] =
    val k = it.knownSize
    if k == 0 || n <= 0 then Iterator.empty
    else if n == 2147483647 then it
    else if k > 0 then it.drop(Math.max(k - n, 0))
    else new TakeRightIterator[A](it, n)
  private[scala] def dropRightIterator[A](it: Iterator[A], n: Int): Iterator[A] =
    if n <= 0 then it
    else
      val k = it.knownSize
      if k >= 0 then it.take(k - n) else new DropRightIterator[A](it, n)

  private final class TakeRightIterator[A](private var underlying: Iterator[A], maxlen: Int) extends Iterator[A]:
    private var len: Int = -1
    private var pos: Int = 0
    private var buf: RawBuffer[A] = null
    def init(): Unit = if buf == null then
      buf = emptyBuffer[A]
      len = 0
      while underlying.hasNext do
        val n = underlying.next()
        if pos >= buf.length then buf.push(n) else buf(pos) = n
        pos += 1
        if pos == maxlen then pos = 0
        len += 1
      underlying = null
      if len > maxlen then len = maxlen
      pos = pos - len
      if pos < 0 then pos += maxlen
    override def knownSize: Int = len
    def hasNext: Boolean =
      init()
      len > 0
    def next(): A =
      init()
      if len == 0 then Iterator.empty[A].next()
      else
        val x = buf(pos)
        pos += 1
        if pos == maxlen then pos = 0
        len -= 1
        x

  private final class DropRightIterator[A](private var underlying: Iterator[A], maxlen: Int) extends Iterator[A]:
    private var len: Int = -1
    private var pos: Int = 0
    private var buf: RawBuffer[A] = null
    def init(): Unit = if buf == null then
      buf = emptyBuffer[A]
      while pos < maxlen && underlying.hasNext do
        buf.push(underlying.next())
        pos += 1
      if !underlying.hasNext then len = 0
      pos = 0
    override def knownSize: Int = len
    def hasNext: Boolean =
      init()
      len != 0
    def next(): A =
      if !hasNext then Iterator.empty[A].next()
      else
        val x = buf(pos)
        if len == -1 then
          buf(pos) = underlying.next()
          if !underlying.hasNext then len = 0
        else len -= 1
        pos += 1
        if pos == maxlen then pos = 0
        x
