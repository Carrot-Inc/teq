package scala

// A radix-balanced trie of 32-way nodes with a tail, scala-library's shape: `apply` and
// `updated` walk one path of log32 n nodes, `:+` copies the tail (at most 32 elements) and
// moves a full one into the trie as a leaf, `+:` and `drop` move the front through `offset`,
// the position of the first element in the trie's index space, and `take` cuts the trie at
// its right edge. Every leaf of the trie is full but the first, whose slots before `offset`
// are unused. No version writes into an array another version can reach: every derived
// version copies what it changes, so two versions derived from one on two threads cannot see
// each other's elements, and a held version retains its own elements and at most one leaf.
// Without a root every element is in `items`, whatever their number: that is the flat array
// `Vector.wrap` makes in O(1) for the std's transient arrays, which no version shares; a
// derived version of a flat vector longer than a leaf goes through the trie the flat vector
// chunks into once, memoised on it.
final class Vector[+A](root: RawBuffer[Any], depth: Int, offset: Int, trieLength: Int, items: RawBuffer[A], itemCount: Int, private var chunked: Vector[Any] = null) extends SeqOps[A, Vector, Vector[A]], IndexedSeq[A]:
  def length: Int = trieLength + itemCount
  def buildCC[B](items: RawBuffer[B]): Vector[B] = Vector.wrap(items)
  // The elements as one array: the tail itself when the vector is flat and ends with it.
  def unsafeArray: RawBuffer[A] =
    if root == null then (if items.length == itemCount then items else arraySlice(items, 0, itemCount))
    else
      val out = emptyBuffer[A]
      foreach(x => out.push(x))
      out
  private def asTrie: Vector[A] =
    if chunked == null then chunked = Vector.chunk(items, itemCount)
    unsafeCast(chunked)
  private def leafAt(p: Int): RawBuffer[A] =
    var node = root
    var level = depth
    while level > 0 do
      node = unsafeCast(node((p >>> (5 * level)) & 31))
      level -= 1
    unsafeCast(node)

  def apply(i: Int): A =
    if i < 0 || i >= length then indexOutOfBounds(i.toString + (if length == 0 then " is out of bounds (empty vector)" else " is out of bounds (min 0, max " + (length - 1).toString + ")"))
    else if i >= trieLength then items(i - trieLength)
    else leafAt(i + offset)((i + offset) & 31)
  def head: A = if length == 0 then noSuchElement("empty.head") else apply(0)
  // scala-library names the empty vector's last by its tail.
  def last: A = if length == 0 then noSuchElement("empty.tail") else apply(length - 1)
  def headOption: Option[A] = if length == 0 then None else Some(apply(0))
  def lastOption: Option[A] = if length == 0 then None else Some(apply(length - 1))
  def tail: Vector[A] = if length == 0 then unsupportedOperation("empty.tail") else drop(1)
  def init: Vector[A] = if length == 0 then unsupportedOperation("empty.init") else take(length - 1)

  def foreach[U](f: A => U): Unit =
    var i = 0
    while i < trieLength do
      val p = i + offset
      val leaf = leafAt(p)
      val from = p & 31
      val until = if from + trieLength - i < 32 then from + trieLength - i else 32
      var j = from
      while j < until do
        f(leaf(j))
        j += 1
      i += until - from
    var j = 0
    while j < itemCount do
      f(items(j))
      j += 1
  override def iterator: Iterator[A] =
    if root == null then Iterator.prefix(items, itemCount)
    else
      var i = 0
      var leaf: RawBuffer[A] = items
      var base = trieLength
      var end = 0
      def step(): A =
        if i >= length then Iterator.exhausted
        else
          if i >= end then
            if i >= trieLength then
              leaf = items
              base = trieLength
              end = length
            else
              val p = i + offset
              leaf = leafAt(p)
              base = i - (p & 31)
              end = if base + 32 < trieLength then base + 32 else trieLength
          i += 1
          leaf(i - 1 - base)
      new FnIterator(() => i < length, () => step())

  def toList: List[A] = fromArray(rawItems(this))
  override def toVector: Vector[A] = this
  override def toIndexedSeq: IndexedSeq[A] = this
  @jvmEvidence
  @jvm("rt $0 $1 rtcall arrayOf(Lscala/IterableOnce;Lscala/reflect/ClassTag;)Ljava/lang/Object; cast_result")
  override def toArray[B >: A: scala.reflect.ClassTag]: Array[B] = unsafeCast(if root == null then arraySlice(items, 0, itemCount) else unsafeArray)

  def map[B](f: A => B): Vector[B] =
    val src = unsafeArray
    val n = length
    val out = emptyBuffer[B]
    var i = 0
    while i < n do
      out.push(f(src(i)))
      i += 1
    Vector.wrap(out)

  def flatMap[B](f: A => IterableOnce[B]): Vector[B] =
    val src = unsafeArray
    val n = length
    val out = emptyBuffer[B]
    var i = 0
    while i < n do
      f(src(i)).foreach(x => out.push(x))
      i += 1
    Vector.wrap(out)

  def filter(p: A => Boolean): Vector[A] =
    val src = unsafeArray
    val n = length
    val out = emptyBuffer[A]
    var i = 0
    while i < n do
      if p(src(i)) then out.push(src(i))
      i += 1
    Vector.wrap(out)

  def filterNot(p: A => Boolean): Vector[A] = filter(x => !p(x))
  def withFilter(p: A => Boolean): Vector[A] = filter(p)

  def foldLeft[B](z: B)(op: (B, A) => B): B =
    var acc = z
    foreach(x => acc = op(acc, x))
    acc

  def reduce[B >: A](op: (B, B) => B): B =
    if length == 0 then unsupportedOperation("empty.reduceLeft")
    else tail.foldLeft[B](head)(op)

  def exists(p: A => Boolean): Boolean =
    val src = unsafeArray
    val n = length
    var i = 0
    var found = false
    while !found && i < n do
      found = p(src(i))
      i += 1
    found

  def forall(p: A => Boolean): Boolean = !exists(x => !p(x))
  def contains[B >: A](elem: B): Boolean = exists(x => x == elem)

  def take(n: Int): Vector[A] =
    if n >= length then this
    else if n <= 0 then Vector.empty
    else if root == null then (if itemCount > 32 then asTrie.take(n) else Vector.wrap(arraySlice(items, 0, n)))
    else if n > trieLength then new Vector(root, depth, offset, trieLength, items, n - trieLength)
    else
      val end = offset + n
      val leafStart = (end - 1) & -32
      if leafStart <= offset then Vector.wrap(arraySlice(leafAt(offset), offset & 31, (offset & 31) + n))
      else
        val last = arraySlice(leafAt(end - 1), 0, ((end - 1) & 31) + 1)
        new Vector(Vector.trimRight(root, depth, leafStart - 1), depth, offset, leafStart - offset, last, last.length)

  def drop(n: Int): Vector[A] =
    if n <= 0 then this
    else if n >= length then Vector.empty
    else if root == null then (if itemCount > 32 then asTrie.drop(n) else Vector.wrap(arraySlice(items, n, itemCount)))
    else if n >= trieLength then Vector.wrap(arraySlice(items, n - trieLength, itemCount))
    else new Vector(Vector.trimLeft(root, depth, offset + n), depth, offset + n, trieLength - n, items, itemCount)

  def slice(from: Int, until: Int): Vector[A] =
    val lo = if from < 0 then 0 else from
    val hi = if until > length then length else until
    if hi <= lo then Vector.empty else drop(lo).take(hi - lo)

  def reverse: Vector[A] =
    val src = unsafeArray
    val out = emptyBuffer[A]
    var i = length - 1
    while i >= 0 do
      out.push(src(i))
      i -= 1
    Vector.wrap(out)

  // An append copies the tail on every target: no version writes into an array another version
  // can reach, a traversal reads no further than its own count, and an array a version holds
  // beyond its count (a `Vector.wrap` of a longer transient array, a `take` of a tail) is never
  // grown under it.
  def :+[B >: A](elem: B): Vector[B] =
    if root == null && itemCount >= 32 then asTrie :+ elem
    else if itemCount < 32 then
      val out: RawBuffer[B] = unsafeCast(arraySlice(items, 0, itemCount))
      out.push(elem)
      new Vector(root, depth, offset, trieLength, out, itemCount + 1)
    else
      val out = emptyBuffer[B]
      out.push(elem)
      val p = offset + trieLength
      val leaf: RawBuffer[Any] = unsafeCast(items)
      if p >= Vector.capacity(depth) then
        val grown = emptyBuffer[Any]
        grown.push(root)
        grown.push(Vector.path(depth, leaf))
        new Vector(grown, depth + 1, offset, trieLength + 32, out, 1)
      else new Vector(Vector.insertLeaf(root, depth, p, leaf), depth, offset, trieLength + 32, out, 1)
  override def appended[B >: A](elem: B): Vector[B] = this :+ elem

  def +:[B >: A](elem: B): Vector[B] =
    if root == null then
      if itemCount >= 32 then asTrie.prepended(elem)
      else
        val out = emptyBuffer[B]
        out.push(elem)
        var i = 0
        while i < itemCount do
          out.push(items(i))
          i += 1
        Vector.wrap(out)
    else if offset == 0 then
      val grown = emptyBuffer[Any]
      grown.push(null)
      grown.push(root)
      new Vector[A](grown, depth + 1, Vector.capacity(depth), trieLength, items, itemCount).prepended(elem)
    else new Vector(Vector.setAt(root, depth, offset - 1, elem), depth, offset - 1, trieLength + 1, items, itemCount)
  override def prepended[B >: A](elem: B): Vector[B] = elem +: this

  def updated[B >: A](index: Int, elem: B): Vector[B] =
    if index < 0 || index >= length then indexOutOfBounds(index.toString + " is out of bounds (min 0, max " + (length - 1).toString + ")")
    else if root == null && itemCount > 32 then asTrie.updated(index, elem)
    else if index >= trieLength then
      val out: RawBuffer[B] = unsafeCast(arraySlice(items, 0, itemCount))
      out(index - trieLength) = elem
      new Vector(root, depth, offset, trieLength, out, itemCount)
    else new Vector(Vector.setAt(root, depth, index + offset, elem), depth, offset, trieLength, items, itemCount)

  override def collectionClassName: String = "Vector"
  override def toString: String = mkString("Vector(", ", ", ")")

object Vector:
  // scala-library's companion is an `IterableFactory`, whose `iterableFactory` a library body
  // names as the implicit `Factory` it resolved.
  implicit def iterableFactory[A]: Factory[A, Vector[A]] =
    new Factory[A, Vector[A]]:
      def fromSpecific(it: IterableOnce[A]): Vector[A] = from(it)
      def newBuilder: scala.collection.mutable.Builder[A, Vector[A]] = Vector.newBuilder[A]
  def wrap[A](items: RawBuffer[A]): Vector[A] = new Vector(null, 0, 0, 0, items, items.length)
  def newBuilder[A]: scala.collection.immutable.VectorBuilder[A] = new scala.collection.immutable.VectorBuilder[A]
  def apply[A](elems: A*): Vector[A] = wrap(iterableToArray(elems))
  def from[A](source: IterableOnce[A]): Vector[A] = source match
    case v: Vector[?] => unsafeCast(v)
    case _ => wrap(iterableToArray(source))
  def empty[A]: Vector[A] = wrap(emptyBuffer[A])
  def fill[A](n: Int)(elem: => A): Vector[A] = wrap(filledBuffer(n)(elem))
  def tabulate[A](n: Int)(f: Int => A): Vector[A] = wrap(tabulatedBuffer(n)(f))

  // The trie over the first `n` of `items`: full leaves of 32 stacked 32 to a node, the rest
  // as the tail.
  def chunk[A](items: RawBuffer[A], n: Int): Vector[A] =
    val leaves = emptyBuffer[Any]
    var i = 0
    while i + 32 <= n do
      leaves.push(arraySlice(items, i, i + 32))
      i += 32
    val rest = arraySlice(items, i, n)
    if leaves.length == 0 then wrap(rest)
    else
      var nodes = leaves
      var depth = 0
      while nodes.length > 1 do
        val up = emptyBuffer[Any]
        var j = 0
        while j < nodes.length do
          up.push(arraySlice(nodes, j, j + 32))
          j += 32
        nodes = up
        depth += 1
      new Vector(unsafeCast(nodes(0)), depth, 0, leaves.length * 32, rest, rest.length)

  // The positions a root `depth` levels above its leaves can hold.
  def capacity(depth: Int): Int = if depth >= 6 then 2147483647 else 1 << (5 * (depth + 1))
  // A chain of `level` single-child nodes down to `leaf`, for a leaf whose position is a
  // multiple of the chain's capacity.
  def path(level: Int, leaf: RawBuffer[Any]): RawBuffer[Any] =
    if level == 0 then leaf
    else
      val node = emptyBuffer[Any]
      node.push(path(level - 1, leaf))
      node
  // The node with `leaf` at position `p`, one past the positions it holds.
  def insertLeaf(node: RawBuffer[Any], level: Int, p: Int, leaf: RawBuffer[Any]): RawBuffer[Any] =
    val idx = (p >>> (5 * level)) & 31
    val out = copyArray(node)
    val child = if level == 1 then leaf else if idx < out.length && out(idx) != null then insertLeaf(unsafeCast(out(idx)), level - 1, p, leaf) else path(level - 1, leaf)
    if idx < out.length then out(idx) = child else out.push(child)
    out
  // The node with `elem` at position `p`, along a copied path; a slot the path lacks (before
  // the front of a vector that grew there) is made with room for its 32 children.
  def setAt(node: RawBuffer[Any], level: Int, p: Int, elem: Any): RawBuffer[Any] =
    val idx = (p >>> (5 * level)) & 31
    val out = copyArray(node)
    if level == 0 then out(idx) = elem
    else out(idx) = if out(idx) == null then padded(level - 1, p, elem) else setAt(unsafeCast(out(idx)), level - 1, p, elem)
    out
  def padded(level: Int, p: Int, elem: Any): RawBuffer[Any] =
    val node = sizedBuffer[Any](32, null)
    val idx = (p >>> (5 * level)) & 31
    node(idx) = if level == 0 then elem else padded(level - 1, p, elem)
    node
  // The node without the positions before `p`.
  def trimLeft(node: RawBuffer[Any], level: Int, p: Int): RawBuffer[Any] =
    val idx = (p >>> (5 * level)) & 31
    val out = copyArray(node)
    var j = 0
    while j < idx do
      out(j) = null
      j += 1
    if level > 0 then out(idx) = trimLeft(unsafeCast(out(idx)), level - 1, p)
    out
  // The node without the positions after `p`.
  def trimRight(node: RawBuffer[Any], level: Int, p: Int): RawBuffer[Any] =
    val idx = (p >>> (5 * level)) & 31
    val out = arraySlice(node, 0, idx + 1)
    if level > 0 then out(idx) = trimRight(unsafeCast(out(idx)), level - 1, p)
    out
