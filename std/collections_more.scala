// Collections of scala-library that library bodies build: `mutable.Stack`, `immutable.LongMap`
// and `util.control.TailCalls`, with scala-library's names, order and printing.
package scala.collection.mutable:

  // The top is the last element of the buffer; the stack iterates from the top.
  final class Stack[A](initialSize: Int = 16) extends SeqOps[A, Stack, Stack[A]], IndexedSeq[A], Builder[A, Stack[A]]:
    private val buf = ArrayBuffer.empty[A]
    def buildCC[B](items: RawBuffer[B]): Stack[B] = Stack.from(Vector.wrap(items))
    def length: Int = buf.length
    def apply(i: Int): A =
      if i < 0 || i >= buf.length then indexOutOfBounds(i.toString)
      else buf(buf.length - 1 - i)
    override def iterator: Iterator[A] =
      val items = rawItems(buf)
      reverseArray(items)
      Iterator.over(items)
    def foreach[U](f: A => U): Unit =
      var i = buf.length - 1
      while i >= 0 do
        f(buf(i))
        i -= 1
    def push(elem: A): this.type =
      buf += elem
      this
    def push(elem1: A, elem2: A, elems: A*): this.type =
      push(elem1)
      push(elem2)
      pushAll(elems)
    def pushAll(elems: IterableOnce[A]): this.type =
      elems.foreach(e => buf += e)
      this
    def pop(): A =
      if buf.length == 0 then noSuchElement("empty collection")
      buf.remove(buf.length - 1)
    def popAll(): Seq[A] =
      val out = toList
      buf.clear()
      out
    def top: A =
      if buf.length == 0 then noSuchElement("empty collection")
      buf.last
    override def head: A = top
    def addOne(elem: A): this.type = push(elem)
    def result(): Stack[A] = this
    def clear(): Unit = buf.clear()
    override def clone(): Stack[A] = new Stack[A]().pushAll(buf)
    override def className: String = "Stack"
    override def toString: String = mkString("Stack(", ", ", ")")

  object Stack:
    def empty[A]: Stack[A] = new Stack[A]()
    // The first element is the top, as scala-library's `Stack(1, 2, 3).top` is 1.
    def apply[A](elems: A*): Stack[A] = from(elems)
    def from[A](elems: IterableOnce[A]): Stack[A] =
      val items = iterableToArray(elems)
      reverseArray(items)
      new Stack[A]().pushAll(Vector.wrap(items))
    def newBuilder[A]: Builder[A, Stack[A]] = new Stack[A]().mapResult(s => from(s.toList.reverse))

package scala.collection.immutable:

  // Entries sorted by their key read as an unsigned number, the order of scala-library's
  // Patricia trie: 0 up to Long.MaxValue, then Long.MinValue up to -1.
  final class LongMap[+T](private val entries: RawBuffer[(Long, Any)]) extends IterableOps[(Long, T), Iterable, LongMap[T]], Iterable[(Long, T)]:
    def buildCC[B](items: RawBuffer[B]): Iterable[B] = fromArray(items)
    override def buildC(items: RawBuffer[(Long, T)]): LongMap[T] = LongMap.from(Vector.wrap(items))
    override def iterator: Iterator[(Long, T)] = Iterator.over(unsafeCast(entries))
    @jvmEvidence
    @jvm("rt $0 $1 rtcall arrayOf(Lscala/IterableOnce;Lscala/reflect/ClassTag;)Ljava/lang/Object; cast_result")
    override def toArray[B >: (Long, T): scala.reflect.ClassTag]: Array[B] = unsafeCast(copyArray(entries))
    override def size: Int = entries.length
    override def knownSize: Int = entries.length
    override def isEmpty: Boolean = entries.length == 0
    def foreach[U](f: ((Long, T)) => U): Unit = entries.foreach(e => f(unsafeCast(e)))
    private def search(key: Long): Int =
      var lo = 0
      var hi = entries.length - 1
      var found = -1
      while found < 0 && lo <= hi do
        val mid = (lo + hi) / 2
        val c = java.lang.Long.compareUnsigned(entries(mid)._1, key)
        if c == 0 then found = mid
        else if c < 0 then lo = mid + 1
        else hi = mid - 1
      if found >= 0 then found else -lo - 1
    def contains(key: Long): Boolean = search(key) >= 0
    def isDefinedAt(key: Long): Boolean = contains(key)
    def get(key: Long): Option[T] =
      val i = search(key)
      if i >= 0 then Some(unsafeCast(entries(i)._2)) else None
    def apply(key: Long): T = get(key) match
      case Some(v) => v
      case None => noSuchElement("key not found: " + key)
    def getOrElse[S >: T](key: Long, default: => S): S = get(key).getOrElse(default)
    def updated[S >: T](key: Long, value: S): LongMap[S] =
      val i = search(key)
      val copy = copyArray[(Long, Any), (Long, Any)](entries)
      if i >= 0 then copy(i) = (key, value)
      else scala.collection.mutable.bufferInsert(copy, -i - 1, (key, value))
      new LongMap(copy)
    def +[S >: T](entry: (Long, S)): LongMap[S] = updated(entry._1, entry._2)
    def removed(key: Long): LongMap[T] =
      val i = search(key)
      if i < 0 then this
      else
        val copy = copyArray[(Long, Any), (Long, Any)](entries)
        scala.collection.mutable.bufferRemove(copy, i)
        new LongMap(copy)
    def -(key: Long): LongMap[T] = removed(key)
    def ++[S >: T](that: IterableOnce[(Long, S)]): LongMap[S] =
      var out: LongMap[S] = this
      that.foreach(e => out = out.updated(e._1, e._2))
      out
    def keys: Iterable[Long] = fromArray(entries.map(e => e._1))
    def keySet: Set[Long] = Set.from(entries.map(e => e._1))
    def values: Iterable[T] = fromArray(entries.map(e => unsafeCast[Any, T](e._2)))
    def keysIterator: Iterator[Long] = Iterator.over(entries.map(e => e._1))
    def valuesIterator: Iterator[T] = Iterator.over(entries.map(e => unsafeCast[Any, T](e._2)))
    def firstKey: Long =
      if entries.length == 0 then noSuchElement("empty.firstKey")
      entries(0)._1
    def lastKey: Long =
      if entries.length == 0 then noSuchElement("empty.lastKey")
      entries(entries.length - 1)._1
    def map[S](f: ((Long, T)) => (Long, S)): LongMap[S] = LongMap.from(Vector.wrap(rawItems(this).map(f)))
    override def map[B](f: ((Long, T)) => B): Iterable[B] = fromArray(rawItems(this).map(f))
    def transform[S](f: (Long, T) => S): LongMap[S] = new LongMap(entries.map(e => (e._1, f(e._1, unsafeCast(e._2)))))
    def mapValues[S](f: T => S): LongMap[S] = new LongMap(entries.map(e => (e._1, f(unsafeCast(e._2)))))
    override def toMap[K2, V2](implicit ev: ((Long, T)) <:< (K2, V2)): scala.Map[K2, V2] = unsafeCast(scala.Map.from(this))
    override def className: String = "LongMap"
    override def mkString(start: String, sep: String, end: String): String =
      joinStrings(Vector.wrap(entries.map(e => "" + e._1 + " -> " + e._2)), start, sep, end)
    override def equals(that: Any): Boolean = that match
      case m: LongMap[?] => scala.Map.from(this).equals(scala.Map.from(m))
      case m: scala.Map[?, ?] => scala.Map.from(this).equals(m)
      case _ => false
    override def hashCode: Int = scala.Map.from(this).hashCode
    override def toString: String = mkString("LongMap(", ", ", ")")

  object LongMap:
    def empty[T]: LongMap[T] = new LongMap[T](emptyBuffer[(Long, Any)])
    def singleton[T](key: Long, value: T): LongMap[T] = empty[T].updated(key, value)
    def apply[T](elems: (Long, T)*): LongMap[T] = from(elems)
    def from[T](elems: IterableOnce[(Long, T)]): LongMap[T] =
      var out = empty[T]
      elems.foreach(e => out = out.updated(e._1, e._2))
      out
    def newBuilder[T]: scala.collection.mutable.Builder[(Long, T), LongMap[T]] =
      scala.collection.mutable.ArrayBuffer.empty[(Long, T)].mapResult(items => from(items))

  // scala-library's `immutable.HashMap`, a `Map` of its own name over the std's persistent map:
  // the operations that give a map back give a `HashMap`.
  final class HashMap[K, +V](private val m: scala.Map[K, V]) extends scala.Map[K, V]:
    def get(key: K): Option[V] = m.get(key)
    def iterator: Iterator[(K, V)] = m.iterator
    override def size: Int = m.size
    override def knownSize: Int = m.size
    override def isEmpty: Boolean = m.isEmpty
    override def contains(key: K): Boolean = m.contains(key)
    override def foreach[U](f: ((K, V)) => U): Unit = m.foreach(f)
    override def buildC(items: RawBuffer[(K, V)]): HashMap[K, V] = new HashMap(scala.Map.from(Vector.wrap(items)))
    def updated[V1 >: V](key: K, value: V1): HashMap[K, V1] = new HashMap(m.updated(key, value))
    def removed(key: K): HashMap[K, V] = new HashMap(m.removed(key))
    override def +[V1 >: V](entry: (K, V1)): HashMap[K, V1] = updated(entry._1, entry._2)
    override def -(key: K): HashMap[K, V] = removed(key)
    override def removedAll(keys: IterableOnce[K]): HashMap[K, V] = new HashMap(m.removedAll(keys))
    override def ++[V1 >: V](that: IterableOnce[(K, V1)]): HashMap[K, V1] = new HashMap(m ++ that)
    override def concat[V1 >: V](that: IterableOnce[(K, V1)]): HashMap[K, V1] = this ++ that
    override def keys: List[K] = m.keys
    override def values: List[V] = m.values
    override def className: String = "HashMap"
    override def toString: String = mkString("HashMap(", ", ", ")")

  object HashMap:
    def empty[K, V]: HashMap[K, V] = new HashMap(scala.Map.empty[K, V])
    def apply[K, V](elems: (K, V)*): HashMap[K, V] = from(elems)
    def from[K, V](elems: IterableOnce[(K, V)]): HashMap[K, V] = elems match
      case h: HashMap[?, ?] => h.asInstanceOf[HashMap[K, V]]
      case _ => new HashMap(scala.Map.from(elems))
    def newBuilder[K, V]: scala.collection.mutable.ReusableBuilder[(K, V), HashMap[K, V]] =
      scala.collection.mutable.ArrayBuffer.empty[(K, V)].mapResult(items => from(items))

package scala.util.control:

  object TailCalls:
    sealed abstract class TailRec[+A]:
      final def map[B](f: A => B): TailRec[B] = flatMap(a => Call(() => Done(f(a))))
      final def flatMap[B](f: A => TailRec[B]): TailRec[B] = this match
        case Done(a) => Call(() => f(a))
        case c: Call[A] => Cont(c, f)
        case c: Cont[?, A] => c.bind(f)
      final def resume: Either[() => TailRec[A], A] =
        var cur: TailRec[Any] = this
        var out: Either[() => TailRec[A], A] | Null = null
        while out == null do
          cur match
            case Done(a) => out = Right(unsafeCast(a))
            case Call(k) => out = Left(unsafeCast(k))
            case c: Cont[?, ?] => cur = c.step
        unsafeCast(out)
      final def result: A =
        var cur: TailRec[Any] = this
        while !cur.isInstanceOf[Done[?]] do
          cur = cur match
            case Call(t) => t()
            case c: Cont[?, ?] => c.step
            case d => d
        unsafeCast(cur.asInstanceOf[Done[Any]].value)

    protected case class Call[A](rest: () => TailRec[A]) extends TailRec[A]
    protected case class Done[A](value: A) extends TailRec[A]
    protected case class Cont[A, B](a: TailRec[A], f: A => TailRec[B]) extends TailRec[B]:
      def bind[C](g: B => TailRec[C]): TailRec[C] = Cont(a, (x: A) => f(x).flatMap(g))
      // One step of `result` on a continuation: what it reduces to next.
      def step: TailRec[B] = a match
        case Done(v) => f(v)
        case Call(t) => t().flatMap(f)
        case c: Cont[?, A] => c.bind(f)

    def tailcall[A](rest: => TailRec[A]): TailRec[A] = Call(() => rest)
    def done[A](result: A): TailRec[A] = Done(result)
