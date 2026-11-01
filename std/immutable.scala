package scala.collection.immutable

// scala-library's `immutable.LinearSeq`: a sequence whose operations go through `head` and `tail`, as
// `Stream` is one.
trait LinearSeq[+A] extends Seq[A]

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
  override def toArray[B >: A: scala.reflect.ClassTag]: Array[B] = arrayByTag(scala.reflect.classTag[B], this, if dead == 0 then unsafeCast(copyArray(entries.unsafeArray)) else unsafeCast(PersistentMap.without(entries, -1)))
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
  override def +(elem: A): SortedSet[A] = incl(elem)
  override def buildC(sorted: RawBuffer[A]): SortedSet[A] = new TreeSet(sorted, ordering)
  override def iterator: Iterator[A] = Iterator.over(items)
  @jvmEvidence
  @jvm("rt $0 $1 rtcall arrayOf(Lscala/IterableOnce;Lscala/reflect/ClassTag;)Ljava/lang/Object; cast_result")
  override def toArray[B >: A: scala.reflect.ClassTag]: Array[B] = arrayByTag(scala.reflect.classTag[B], this, unsafeCast(copyArray(items)))
  override def size: Int = items.length
  // scala-library's `TreeSet` keeps `IterableOnce`'s -1, its size a count of its tree.
  override def knownSize: Int = -1
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
      // The element sought against the set's, as scala-library's tree compares (`RB.lookup`, `RB.update`).
      val c = ordering.compare(elem, items(mid))
      if c == 0 then found = mid
      else if c > 0 then lo = mid + 1
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
  // scala-library's `StrictOptimizedSortedSetOps`: each result is added to the result's builder as it is made
  // (`strictOptimizedMap`, `strictOptimizedFlatMap` through the builder's `++=`, `strictOptimizedCollect`,
  // `strictOptimizedZip`), its ordering run then, before the next element is taken. Called through the static type
  // Set, no ordering is passed and the result is a plain Set.
  override def map[B](f: A => B)(implicit ord: Ordering[B]): SortedSet[B] =
    val b = SortedSet.builderOrPlain(ord)
    foreach(x => b.addOne(f(x)))
    b.result()
  override def flatMap[B](f: A => IterableOnce[B])(implicit ord: Ordering[B]): SortedSet[B] =
    val b = SortedSet.builderOrPlain(ord)
    foreach(x => b ++= f(x))
    b.result()
  override def collect[B](pf: PartialFunction[A, B])(implicit ord: Ordering[B]): SortedSet[B] =
    val b = SortedSet.builderOrPlain(ord)
    val miss = partialMiss[A, B]
    foreach: x =>
      val v = pf.applyOrElse(x, miss)
      if !isPartialMiss(v) then b.addOne(v)
    b.result()
  override def zip[B](that: IterableOnce[B])(implicit ord: Ordering[(A, B)]): SortedSet[(A, B)] =
    val b = SortedSet.builderOrPlain(ord)
    val a = iterator
    val c = that.iterator
    while a.hasNext && c.hasNext do b.addOne((a.next(), c.next()))
    b.result()
  override def collectionClassName: String = "TreeSet"
  override def toString: String = mkString("TreeSet(", ", ", ")")

object SortedSet extends scala.collection.EvidenceIterableFactory[SortedSet, Ordering]:
  // The builder of a result: a sorted set's where the ordering is given, a plain Set's where a call through the
  // static type Set passed none.
  def builderOrPlain[B](ord: Ordering[B]): scala.collection.mutable.Builder[B, SortedSet[B]] =
    if js.isUndefined(ord) then unsafeCast(Set.newBuilder[B]) else newBuilder(using ord)
  def empty[A](implicit ord: Ordering[A]): SortedSet[A] = new TreeSet(emptyBuffer[A], ord)
  def apply[A](elems: A*)(implicit ord: Ordering[A]): SortedSet[A] = from(elems)
  // scala-library's `TreeSet.from`: a sorted set of the same ordering is the set itself, every sorted
  // set being a `TreeSet` here.
  def from[A](elems: IterableOnce[A])(implicit ord: Ordering[A]): TreeSet[A] = elems match
    case ts: SortedSet[A] @unchecked if ord == ts.ordering => unsafeCast(ts)
    case _ =>
      val sorted = iterableToArray(elems)
      sortArray(sorted, (a, b) => ord.compare(a, b))
      val out = emptyBuffer[A]
      sorted.foreach(x => if out.length == 0 || ord.compare(out(out.length - 1), x) != 0 then out.push(x))
      new TreeSet(out, ord)
  def newBuilder[A](implicit ord: Ordering[A]): scala.collection.mutable.Builder[A, SortedSet[A]] = TreeSet.newBuilder[A]

// Every sorted set is a TreeSet, as in scala-library; the class keeps the two types distinct
// for a library's givens per set class (zio-json's `sortedSet` and `treeSet`).
final class TreeSet[A](items: RawBuffer[A], ordering: Ordering[A]) extends SortedSet[A](items, ordering):
  // The tree scala-library's set is, which a builder's `addAll` adopts or unites with: the one a builder published,
  // else built of the array without comparing (`fromOrderedKeys`), once.
  private[immutable] var rb: RBTree[A, Any] = null
  private[immutable] def rbTree: RBTree[A, Any] =
    if (rb eq null) && items.length > 0 then rb = RedBlackTree.fromOrderedKeys(items)
    rb
  override def incl(elem: A): TreeSet[A] = unsafeCast(super.incl(elem))
  override def excl(elem: A): TreeSet[A] = unsafeCast(super.excl(elem))
  override def ++(that: IterableOnce[A]): TreeSet[A] = unsafeCast(super.++(that))
  override def concat(that: IterableOnce[A]): TreeSet[A] = unsafeCast(super.concat(that))
  override def removedAll(that: IterableOnce[A]): TreeSet[A] = unsafeCast(super.removedAll(that))
  override def +(elem: A): TreeSet[A] = incl(elem)
  override def -(elem: A): TreeSet[A] = excl(elem)
  override def map[B](f: A => B)(implicit ord: Ordering[B]): TreeSet[B] = unsafeCast(super.map(f)(using ord))
  override def flatMap[B](f: A => IterableOnce[B])(implicit ord: Ordering[B]): TreeSet[B] = unsafeCast(super.flatMap(f)(using ord))
  override def collect[B](pf: PartialFunction[A, B])(implicit ord: Ordering[B]): TreeSet[B] = unsafeCast(super.collect(pf)(using ord))
  override def zip[B](that: IterableOnce[B])(implicit ord: Ordering[(A, B)]): TreeSet[(A, B)] = unsafeCast(super.zip(that)(using ord))

object TreeSet:
  def empty[A](implicit ord: Ordering[A]): TreeSet[A] = new TreeSet(emptyBuffer[A], ord)
  def apply[A](elems: A*)(implicit ord: Ordering[A]): TreeSet[A] = from(elems)
  def from[A](elems: IterableOnce[A])(implicit ord: Ordering[A]): TreeSet[A] = SortedSet.from(elems)
  def newBuilder[A](implicit ord: Ordering[A]): scala.collection.mutable.ReusableBuilder[A, TreeSet[A]] = new TreeSetBuilder(ord)

  // scala-library's `TreeSetBuilder`, over `SetHelper`: each element inserted as it is added (`mutableUpd`), its
  // ordering run then, before the next one is made; `addAll` of a set or a map of the same ordering adopting its tree
  // where nothing was added yet and uniting with it otherwise, any other source element by element; a result
  // publishes the tree (`beforePublish`) and leaves the builder as it was.
  private[immutable] final class TreeSetBuilder[A](ord: Ordering[A]) extends scala.collection.mutable.ReusableBuilder[A, TreeSet[A]]:
    private var tree: RBTree[A, Any] = null
    def addOne(elem: A): this.type =
      tree = RedBlackTree.setUpd(tree, elem, ord)
      this
    override def addAll(xs: IterableOnce[A]): this.type =
      xs match
        case ts: TreeSet[A] @unchecked if ts.ordering == ord =>
          if tree eq null then tree = ts.rbTree
          else tree = RedBlackTree.union(RedBlackTree.beforePublish(tree), ts.rbTree, ord)
        case tm: TreeMap[A, ?] @unchecked if tm.ordering == ord =>
          if tree eq null then tree = tm.rbTree
          else tree = RedBlackTree.union(RedBlackTree.beforePublish(tree), tm.rbTree, ord)
        case _ =>
          super.addAll(xs)
      this
    def clear(): Unit = tree = null
    def result(): TreeSet[A] =
      val published = RedBlackTree.beforePublish(tree)
      val items = emptyBuffer[A]
      RedBlackTree.keysInto(published, items)
      val out = new TreeSet(items, ord)
      out.rb = published
      out

// scala-library's immutable `RedBlackTree`, the part the sorted collections' builders run: a node mutable until it is
// published (`beforePublish`), the builders' insertion (`SetHelper.mutableUpd`, `MapHelper.mutableUpd`) with its
// balancing, `union` with its `split` and `join`, and `fromOrderedKeys` / `fromOrderedEntries` for a set or a map the
// std keeps as a sorted array. A builder compares as scala-library's does, the same elements in the same order,
// against a tree of the same shape; its result is the std's array again, read out of the tree in order.
private[immutable] final class RBTree[A, B](
    var key: A, var value: B, var left: RBTree[A, B], var right: RBTree[A, B], var red: Boolean, var mutable: Boolean)

private[immutable] object RedBlackTree:
  def isRedTree[A, B](t: RBTree[A, B]): Boolean = (t ne null) && t.red
  def isBlackTree[A, B](t: RBTree[A, B]): Boolean = (t ne null) && !t.red

  // The immutable nodes `union` makes (`RedTree`, `BlackTree`, `mkTree`), and the mutable one an insertion makes.
  def RedTree[A, B](key: A, value: B, left: RBTree[A, B], right: RBTree[A, B]): RBTree[A, B] =
    new RBTree(key, value, left, right, true, false)
  def BlackTree[A, B](key: A, value: B, left: RBTree[A, B], right: RBTree[A, B]): RBTree[A, B] =
    new RBTree(key, value, left, right, false, false)
  def mkTree[A, B](isBlack: Boolean, key: A, value: B, left: RBTree[A, B], right: RBTree[A, B]): RBTree[A, B] =
    new RBTree(key, value, left, right, !isBlack, false)
  def mutableRedTree[A, B](key: A, value: B): RBTree[A, B] = new RBTree(key, value, null, null, true, true)

  def black[A, B](t: RBTree[A, B]): RBTree[A, B] =
    if !t.red then t else new RBTree(t.key, t.value, t.left, t.right, false, t.mutable)
  def blacken[A, B](t: RBTree[A, B]): RBTree[A, B] = if t eq null then null else black(t)

  // `Tree`'s mutable operations: a mutable node changes in place, an immutable (published) one is copied, mutable.
  def makeImmutable[A, B](t: RBTree[A, B]): RBTree[A, B] =
    if t.mutable then
      if t.left ne null then makeImmutable(t.left)
      if t.right ne null then makeImmutable(t.right)
      t.mutable = false
    t
  def mutableBlack[A, B](t: RBTree[A, B]): RBTree[A, B] =
    if !t.red then t
    else if t.mutable then
      t.red = false
      t
    else new RBTree(t.key, t.value, t.left, t.right, false, true)
  def mutableWithV[A, B](t: RBTree[A, B], v: B): RBTree[A, B] =
    if v.asInstanceOf[AnyRef] eq t.value.asInstanceOf[AnyRef] then t
    else if t.mutable then
      t.value = v
      t
    else new RBTree(t.key, v, t.left, t.right, t.red, true)
  def mutableWithLeft[A, B](t: RBTree[A, B], l: RBTree[A, B]): RBTree[A, B] =
    if t.left eq l then t
    else if t.mutable then
      t.left = l
      t
    else new RBTree(t.key, t.value, l, t.right, t.red, true)
  def mutableWithRight[A, B](t: RBTree[A, B], r: RBTree[A, B]): RBTree[A, B] =
    if t.right eq r then t
    else if t.mutable then
      t.right = r
      t
    else new RBTree(t.key, t.value, t.left, r, t.red, true)
  def mutableWithLeftRight[A, B](t: RBTree[A, B], l: RBTree[A, B], r: RBTree[A, B]): RBTree[A, B] =
    if (t.left eq l) && (t.right eq r) then t
    else if t.mutable then
      t.left = l
      t.right = r
      t
    else new RBTree(t.key, t.value, l, r, t.red, true)
  def mutableBlackWithLeft[A, B](t: RBTree[A, B], l: RBTree[A, B]): RBTree[A, B] =
    if (t.left eq l) && !t.red then t
    else if t.mutable then
      t.red = false
      t.left = l
      t
    else new RBTree(t.key, t.value, l, t.right, false, true)
  def mutableBlackWithRight[A, B](t: RBTree[A, B], r: RBTree[A, B]): RBTree[A, B] =
    if (t.right eq r) && !t.red then t
    else if t.mutable then
      t.red = false
      t.right = r
      t
    else new RBTree(t.key, t.value, t.left, r, false, true)

  // `Helper.beforePublish`: the tree a result holds, immutable from then on.
  def beforePublish[A, B](t: RBTree[A, B]): RBTree[A, B] =
    if t eq null then t
    else if t.mutable then makeImmutable(mutableBlack(t))
    else black(t)

  // `Helper.mutableBalanceLeft`: `newLeft` in the place of `tree.left`, balanced.
  def mutableBalanceLeft[A, B](tree: RBTree[A, B], newLeft: RBTree[A, B]): RBTree[A, B] =
    if newLeft.red then
      val nll = newLeft.left
      val nlr = newLeft.right
      if isRedTree(nll) then
        val resultLeft = mutableBlack(nll)
        val resultRight = mutableBlackWithLeft(tree, nlr)
        mutableWithLeftRight(newLeft, resultLeft, resultRight)
      else if isRedTree(nlr) then
        val nlrr = nlr.right
        val resultLeft = mutableBlackWithRight(newLeft, nlr.left)
        val resultRight = mutableBlackWithLeft(tree, nlrr)
        mutableWithLeftRight(nlr, resultLeft, resultRight)
      else mutableWithLeft(tree, newLeft)
    else mutableWithLeft(tree, newLeft)

  // `Helper.mutableBalanceRight`: `newRight` in the place of `tree.right`, balanced.
  def mutableBalanceRight[A, B](tree: RBTree[A, B], newRight: RBTree[A, B]): RBTree[A, B] =
    if newRight.red then
      val nrl = newRight.left
      if isRedTree(nrl) then
        val resultLeft = mutableBlackWithRight(tree, nrl.left)
        val resultRight = mutableBlackWithLeft(newRight, nrl.right)
        mutableWithLeftRight(nrl, resultLeft, resultRight)
      else
        val nrr = newRight.right
        if isRedTree(nrr) then
          val resultLeft = mutableBlackWithRight(tree, nrl)
          val resultRight = mutableBlack(nrr)
          mutableWithLeftRight(newRight, resultLeft, resultRight)
        else mutableWithRight(tree, newRight)
    else mutableWithRight(tree, newRight)

  // `SetHelper.mutableUpd`: an empty tree a new node; the element itself (by reference) the tree as it was; else the
  // new element compared with the node's, left or right, and a node it equals kept.
  def setUpd[A](tree: RBTree[A, Any], k: A, ord: Ordering[A]): RBTree[A, Any] =
    if tree eq null then mutableRedTree[A, Any](k, ())
    else if k.asInstanceOf[AnyRef] eq tree.key.asInstanceOf[AnyRef] then tree
    else
      val cmp = ord.compare(k, tree.key)
      if cmp < 0 then mutableBalanceLeft(tree, setUpd(tree.left, k, ord))
      else if cmp > 0 then mutableBalanceRight(tree, setUpd(tree.right, k, ord))
      else tree

  // `MapHelper.mutableUpd`: as `setUpd`, the value taken where the key is found.
  def mapUpd[A, B](tree: RBTree[A, B], k: A, v: B, ord: Ordering[A]): RBTree[A, B] =
    if tree eq null then mutableRedTree(k, v)
    else if k.asInstanceOf[AnyRef] eq tree.key.asInstanceOf[AnyRef] then mutableWithV(tree, v)
    else
      val cmp = ord.compare(k, tree.key)
      if cmp < 0 then mutableBalanceLeft(tree, mapUpd(tree.left, k, v, ord))
      else if cmp > 0 then mutableBalanceRight(tree, mapUpd(tree.right, k, v, ord))
      else mutableWithV(tree, v)

  // `union` ("Just Join for Parallel Ordered Sets"): the first tree split at each key of the second, joined again.
  def union[A, B](t1: RBTree[A, B], t2: RBTree[A, B], ord: Ordering[A]): RBTree[A, B] = blacken(unionRec(t1, t2, ord))
  private def rank[A, B](t: RBTree[A, B], bh: Int): Int =
    if t eq null then 0 else if !t.red then 2 * (bh - 1) else 2 * bh - 1
  private def joinRight[A, B](tl: RBTree[A, B], k: A, v: B, tr: RBTree[A, B], bhtl: Int, rtr: Int): RBTree[A, B] =
    val rtl = rank(tl, bhtl)
    if rtl == (rtr / 2) * 2 then RedTree(k, v, tl, tr)
    else
      val tlBlack = isBlackTree(tl)
      val bhtlr = if tlBlack then bhtl - 1 else bhtl
      val ttr = joinRight(tl.right, k, v, tr, bhtlr, rtr)
      if tlBlack && isRedTree(ttr) && isRedTree(ttr.right) then
        RedTree(ttr.key, ttr.value, BlackTree(tl.key, tl.value, tl.left, ttr.left), black(ttr.right))
      else mkTree(tlBlack, tl.key, tl.value, tl.left, ttr)
  private def joinLeft[A, B](tl: RBTree[A, B], k: A, v: B, tr: RBTree[A, B], rtl: Int, bhtr: Int): RBTree[A, B] =
    val rtr = rank(tr, bhtr)
    if rtr == (rtl / 2) * 2 then RedTree(k, v, tl, tr)
    else
      val trBlack = isBlackTree(tr)
      val bhtrl = if trBlack then bhtr - 1 else bhtr
      val ttl = joinLeft(tl, k, v, tr.left, rtl, bhtrl)
      if trBlack && isRedTree(ttl) && isRedTree(ttl.left) then
        RedTree(ttl.key, ttl.value, black(ttl.left), BlackTree(tr.key, tr.value, ttl.right, tr.right))
      else mkTree(trBlack, tr.key, tr.value, ttl, tr.right)
  private def blackHeight[A, B](t: RBTree[A, B], i: Int): Int =
    if t eq null then i + 1 else blackHeight(t.left, if !t.red then i + 1 else i)
  private def join[A, B](tl: RBTree[A, B], k: A, v: B, tr: RBTree[A, B]): RBTree[A, B] =
    val bhtl = blackHeight(tl, 0)
    val bhtr = blackHeight(tr, 0)
    if bhtl > bhtr then
      val tt = joinRight(tl, k, v, tr, bhtl, rank(tr, bhtr))
      if isRedTree(tt) && isRedTree(tt.right) then black(tt) else tt
    else if bhtr > bhtl then
      val tt = joinLeft(tl, k, v, tr, rank(tl, bhtl), bhtr)
      if isRedTree(tt) && isRedTree(tt.left) then black(tt) else tt
    else mkTree(isRedTree(tl) || isRedTree(tr), k, v, tl, tr)
  private def split[A, B](t: RBTree[A, B], k2: A, ord: Ordering[A]): (RBTree[A, B], RBTree[A, B], RBTree[A, B], A) =
    if t eq null then (null, null, null, k2)
    else
      val cmp = ord.compare(k2, t.key)
      if cmp == 0 then (t.left, t, t.right, t.key)
      else if cmp < 0 then
        val (ll, b, lr, k1) = split(t.left, k2, ord)
        (ll, b, join(lr, t.key, t.value, t.right), k1)
      else
        val (rl, b, rr, k1) = split(t.right, k2, ord)
        (join(t.left, t.key, t.value, rl), b, rr, k1)
  private def unionRec[A, B](t1: RBTree[A, B], t2: RBTree[A, B], ord: Ordering[A]): RBTree[A, B] =
    if (t1 eq null) || (t1 eq t2) then t2
    else if t2 eq null then t1
    else
      val (l1, _, r1, k1) = split(t1, t2.key, ord)
      val tl = unionRec(l1, t2.left, ord)
      val tr = unionRec(r1, t2.right, ord)
      join(tl, k1, t2.value, tr)

  // `fromOrderedKeys` / `fromOrderedEntries`: the tree scala-library builds of a sorted sequence, without comparing.
  def fromOrderedKeys[A](items: RawBuffer[A]): RBTree[A, Any] =
    val n = items.length
    val maxUsedDepth = 32 - java.lang.Integer.numberOfLeadingZeros(n)
    var next = 0
    def f(level: Int, size: Int): RBTree[A, Any] =
      if size == 0 then null
      else if size == 1 then
        val x = items(next)
        next += 1
        mkTree[A, Any](level != maxUsedDepth || level == 1, x, null, null, null)
      else
        val leftSize = (size - 1) / 2
        val left = f(level + 1, leftSize)
        val x = items(next)
        next += 1
        val right = f(level + 1, size - 1 - leftSize)
        BlackTree[A, Any](x, null, left, right)
    f(1, n)
  def fromOrderedEntries[A](entries: RawBuffer[(A, Any)]): RBTree[A, Any] =
    val n = entries.length
    val maxUsedDepth = 32 - java.lang.Integer.numberOfLeadingZeros(n)
    var next = 0
    def f(level: Int, size: Int): RBTree[A, Any] =
      if size == 0 then null
      else if size == 1 then
        val e = entries(next)
        next += 1
        mkTree[A, Any](level != maxUsedDepth || level == 1, e._1, e._2, null, null)
      else
        val leftSize = (size - 1) / 2
        val left = f(level + 1, leftSize)
        val e = entries(next)
        next += 1
        val right = f(level + 1, size - 1 - leftSize)
        BlackTree[A, Any](e._1, e._2, left, right)
    f(1, n)

  // The keys, or the entries, of a tree in order: the std's array of a result.
  def keysInto[A, B](t: RBTree[A, B], out: RawBuffer[A]): Unit =
    if t ne null then
      keysInto(t.left, out)
      out.push(t.key)
      keysInto(t.right, out)
  def entriesInto[A, B](t: RBTree[A, B], out: RawBuffer[(A, Any)]): Unit =
    if t ne null then
      entriesInto(t.left, out)
      out.push((t.key, t.value))
      entriesInto(t.right, out)

// Insertion-ordered, as scala-library's ListSet iterates: an array with the elements in the
// order they came, which an update copies like the other sets do.
final class ListSet[A](private val items: RawBuffer[A]) extends SetOps[A, ListSet[A]], IterableOps[A, Set, ListSet[A]], Set[A]:
  override def buildC(elems: RawBuffer[A]): ListSet[A] = ListSet.from(Vector.wrap(elems))
  override def iterator: Iterator[A] = Iterator.over(items)
  @jvmEvidence
  @jvm("rt $0 $1 rtcall arrayOf(Lscala/IterableOnce;Lscala/reflect/ClassTag;)Ljava/lang/Object; cast_result")
  override def toArray[B >: A: scala.reflect.ClassTag]: Array[B] = arrayByTag(scala.reflect.classTag[B], this, unsafeCast(copyArray(items)))
  override def size: Int = items.length
  // scala-library's: the empty set's 0, a node's -1.
  override def knownSize: Int = if items.length == 0 then 0 else -1
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
// scala-library's `immutable.Iterable`, the std's `Iterable` (`Future.foldLeft` takes one).
type Iterable[+A] = scala.Iterable[A]

object ArraySeq:
  def apply[A](elems: A*): scala.ArraySeq[A] = scala.ArraySeq(elems*)
  def empty[A]: scala.ArraySeq[A] = scala.ArraySeq.empty[A]
  def from[A](source: IterableOnce[A]): scala.ArraySeq[A] = scala.ArraySeq.from(source)
  def unsafeWrapArray[A](x: Array[A]): scala.ArraySeq[A] = scala.ArraySeq.unsafeWrapArray(x)

// A queue over an array copied on each update, as the other immutable sequences here are:
// scala-library's two-list queue has the same operations and order.
sealed class Queue[+A](private val items: RawBuffer[A]) extends SeqOps[A, Queue, Queue[A]], Seq[A]:
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
  override def toArray[B >: A: scala.reflect.ClassTag]: Array[B] = arrayByTag(scala.reflect.classTag[B], this, unsafeCast(copyArray(items)))
  override def collectionClassName: String = "Queue"
  override def toString: String = mkString("Queue(", ", ", ")")

object Queue extends IterableFactory[Queue]:
  override def apply[A](elems: A*): Queue[A] = new Queue(iterableToArray(elems))
  def empty[A]: Queue[A] = new Queue(emptyBuffer[A])
  def from[A](source: IterableOnce[A]): Queue[A] = new Queue(iterableToArray(source))
  def newBuilder[A]: scala.collection.mutable.Builder[A, Queue[A]] =
    scala.collection.mutable.ArrayBuffer.empty[A].mapResult(items => from(items))

// ListMap is a class of its own over the std's insertion-ordered Map: a library's givens per
// map class (zio-json's `map` and `listMap`) need the two types distinct.
final class ListMap[K, +V](private val underlying: Map[K, V]) extends Map[K, V]:
  override def size: Int = underlying.size
  // scala-library's: the empty map's 0, a node's -1.
  override def knownSize: Int = if underlying.isEmpty then 0 else -1
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
  // Reached through Iterable, a transform may give a List rather than a Map (`Map.fromResults`): the
  // value is tested as any value, whose class a type pattern over `Map` would take for granted.
  def wrapResults[K, V](m: Map[K, V]): Map[K, V] = (m: Any) match
    case m: Map[?, ?] => wrap(unsafeCast(m))
    case _ => m
  def newBuilder[K, V]: scala.collection.mutable.ReusableBuilder[(K, V), ListMap[K, V]] =
    scala.collection.mutable.ArrayBuffer.empty[(K, V)].mapResult(from)

// A sorted array of entries: lookups are binary searches, updates copy the array like SortedSet.
class SortedMap[K, +V](private val entries: RawBuffer[(K, Any)], val ordering: Ordering[K]) extends IterableOps[(K, V), Iterable, SortedMap[K, V]], scala.collection.Map[K, V]:
  // `collection.Map`'s, of this map's kind: `IterableOps`'s bodies, written out rather than called
  // through `super`, a call the reach answers with the member of that name of every class.
  override def filter(p: ((K, V)) => Boolean): SortedMap[K, V] =
    val out = emptyBuffer[(K, V)]
    foreach(x => if p(x) then out.push(x))
    buildC(out)
  override def tail: SortedMap[K, V] =
    if isEmpty then unsupportedOperation(null) else drop(1)
  def buildCC[B](items: RawBuffer[B]): Iterable[B] = fromArray(items)
  override def buildC(items: RawBuffer[(K, V)]): SortedMap[K, V] = SortedMap.from(Vector.wrap(items))(using ordering)
  override def iterator: Iterator[(K, V)] = Iterator.over(unsafeCast(entries))
  @jvmEvidence
  @jvm("rt $0 $1 rtcall arrayOf(Lscala/IterableOnce;Lscala/reflect/ClassTag;)Ljava/lang/Object; cast_result")
  override def toArray[B >: (K, V): scala.reflect.ClassTag]: Array[B] = arrayByTag(scala.reflect.classTag[B], this, unsafeCast(copyArray(entries)))
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
  override def contains(key: K): Boolean = search(key) >= 0
  override def isDefinedAt(key: K): Boolean = contains(key)
  def get(key: K): Option[V] =
    val i = search(key)
    if i >= 0 then Some(unsafeCast(entries(i)._2)) else None
  override def apply(key: K): V = get(key) match
    case Some(v) => v
    case None => noSuchElement("key not found: " + key.toString)
  override def getOrElse[V1 >: V](key: K, default: => V1): V1 = get(key).getOrElse(default)
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
  // scala-library's `StrictOptimizedSortedMapOps`: each result is added to the result's builder as it is made
  // (`strictOptimizedMap`, `strictOptimizedCollect`, `strictOptimizedFlatMap` through the builder's `++=`), its
  // ordering run then, before the next entry is taken; a call that passes no ordering builds a plain Map.
  def map[K2, V2](f: ((K, V)) => (K2, V2))(implicit ord: Ordering[K2]): SortedMap[K2, V2] =
    val b = SortedMap.builderOrPlain[K2, V2](ord)
    foreach(e => b.addOne(f(e)))
    b.result()
  override def map[B](f: ((K, V)) => B): Iterable[B] = fromArray(rawItems(this).map(f))
  def collect[K2, V2](pf: PartialFunction[(K, V), (K2, V2)])(implicit ord: Ordering[K2]): SortedMap[K2, V2] =
    val b = SortedMap.builderOrPlain[K2, V2](ord)
    val miss = partialMiss[(K, V), (K2, V2)]
    foreach: e =>
      val v = pf.applyOrElse(e, miss)
      if !isPartialMiss(v) then b.addOne(v)
    b.result()
  override def collect[B](pf: PartialFunction[(K, V), B]): Iterable[B] = fromArray(Vector.wrap(rawItems(this)).collect(pf).unsafeArray)
  def flatMap[K2, V2](f: ((K, V)) => IterableOnce[(K2, V2)])(implicit ord: Ordering[K2]): SortedMap[K2, V2] =
    val b = SortedMap.builderOrPlain[K2, V2](ord)
    foreach(e => b ++= f(e))
    b.result()
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
  override def collectionClassName: String = "TreeMap"
  override def toString: String = mkString("TreeMap(", ", ", ")")

object SortedMap:
  // The builder of a result: a sorted map's where the ordering is given, a plain Map's where a call passed none.
  def builderOrPlain[K, V](ord: Ordering[K]): scala.collection.mutable.Builder[(K, V), SortedMap[K, V]] =
    if js.isUndefined(ord) then unsafeCast(scala.Map.newBuilder[K, V]) else TreeMap.newBuilder[K, V](using ord)
  def empty[K, V](implicit ord: Ordering[K]): SortedMap[K, V] = new TreeMap(emptyBuffer[(K, Any)], ord)
  def apply[K, V](entries: (K, V)*)(implicit ord: Ordering[K]): SortedMap[K, V] = from(entries)
  // scala-library's `TreeMap.from`: a sorted map of the same ordering is the map itself, every sorted
  // map being a `TreeMap` here.
  def from[K, V](entries: IterableOnce[(K, V)])(implicit ord: Ordering[K]): SortedMap[K, V] = entries match
    case sm: SortedMap[K, V] @unchecked if ord == sm.ordering => sm
    case _ =>
      var out: SortedMap[K, V] = empty[K, V]
      entries.foreach(e => out = out.updated(e._1, e._2))
      out
  def newBuilder[K, V](implicit ord: Ordering[K]): scala.collection.mutable.Builder[(K, V), SortedMap[K, V]] = TreeMap.newBuilder[K, V]

// Every sorted map is a TreeMap, as in scala-library; the class keeps the two types distinct.
final class TreeMap[K, +V](entries: RawBuffer[(K, Any)], ordering: Ordering[K]) extends SortedMap[K, V](entries, ordering):
  // The tree scala-library's map is (`tree0`), which a builder's `addAll` adopts or unites with: the one a builder
  // published, else built of the array without comparing (`fromOrderedEntries`), once.
  private[immutable] var rb: RBTree[K, Any] = null
  private[immutable] def rbTree: RBTree[K, Any] =
    if (rb eq null) && entries.length > 0 then rb = RedBlackTree.fromOrderedEntries(entries)
    rb
  override def updated[V1 >: V](key: K, value: V1): TreeMap[K, V1] = unsafeCast(super.updated(key, value))
  override def +[V1 >: V](entry: (K, V1)): TreeMap[K, V1] = unsafeCast(super.+(entry))
  override def removed(key: K): TreeMap[K, V] = unsafeCast(super.removed(key))
  override def -(key: K): TreeMap[K, V] = unsafeCast(super.-(key))
  override def ++[V1 >: V](that: IterableOnce[(K, V1)]): TreeMap[K, V1] = unsafeCast(super.++(that))
  override def concat[V1 >: V](that: IterableOnce[(K, V1)]): TreeMap[K, V1] = unsafeCast(super.concat(that))
  override def --(keys: IterableOnce[K]): TreeMap[K, V] = unsafeCast(super.--(keys))
  // scala-library's `TreeMap` is a `SortedMapOps[K, V, TreeMap, TreeMap[K, V]]`: what it maps
  // to pairs is a `TreeMap`.
  override def map[K2, V2](f: ((K, V)) => (K2, V2))(implicit ord: Ordering[K2]): TreeMap[K2, V2] = unsafeCast(super.map(f)(using ord))
  override def collect[K2, V2](pf: PartialFunction[(K, V), (K2, V2)])(implicit ord: Ordering[K2]): TreeMap[K2, V2] = unsafeCast(super.collect(pf)(using ord))
  override def flatMap[K2, V2](f: ((K, V)) => IterableOnce[(K2, V2)])(implicit ord: Ordering[K2]): TreeMap[K2, V2] = unsafeCast(super.flatMap(f)(using ord))

object TreeMap:
  def empty[K, V](implicit ord: Ordering[K]): TreeMap[K, V] = new TreeMap(emptyBuffer[(K, Any)], ord)
  def apply[K, V](entries: (K, V)*)(implicit ord: Ordering[K]): TreeMap[K, V] = from(entries)
  def from[K, V](entries: IterableOnce[(K, V)])(implicit ord: Ordering[K]): TreeMap[K, V] = unsafeCast(SortedMap.from(entries))
  def newBuilder[K, V](implicit ord: Ordering[K]): scala.collection.mutable.ReusableBuilder[(K, V), TreeMap[K, V]] = new TreeMapBuilder(ord)

  // scala-library's `TreeMapBuilder`, over `MapHelper`: each entry inserted as it is added (`mutableUpd`, a key found
  // taking the new value), its ordering run then; `addAll` of a map of the same ordering adopting its tree where nothing
  // was added yet and uniting with it otherwise, of another map entry by entry (`adder.addForEach`), any other source
  // element by element; a result publishes the tree (`beforePublish`) and leaves the builder as it was.
  private[immutable] final class TreeMapBuilder[K, V](ord: Ordering[K]) extends scala.collection.mutable.ReusableBuilder[(K, V), TreeMap[K, V]]:
    private var tree: RBTree[K, Any] = null
    def addOne(elem: (K, V)): this.type =
      tree = RedBlackTree.mapUpd[K, Any](tree, elem._1, elem._2, ord)
      this
    // scala-library's `adder`, one for the builder's life: `addForEach` starts its accumulator at the builder's tree,
    // runs the map's `foreachEntry` (its own walk where it has one) into it, and commits it once all are in, then
    // empties it. A nested `addAll` of a map from inside that walk starts, commits and empties the same accumulator.
    private var accumulator: RBTree[K, Any] = null
    private val adder: (K, V) => Unit = (k, v) => accumulator = RedBlackTree.mapUpd[K, Any](accumulator, k, v, ord)
    private def addForEach(hasForEach: scala.collection.Map[K, V]): Unit =
      accumulator = tree
      hasForEach.foreachEntry(adder)
      tree = accumulator
      accumulator = null
    override def addAll(xs: IterableOnce[(K, V)]): this.type =
      xs match
        case ts: TreeMap[K, V] @unchecked if ts.ordering == ord =>
          if tree eq null then tree = ts.rbTree
          else tree = RedBlackTree.union(RedBlackTree.beforePublish(tree), ts.rbTree, ord)
        case that: scala.collection.Map[K, V] @unchecked =>
          addForEach(that)
        case _ =>
          super.addAll(xs)
      this
    def clear(): Unit = tree = null
    def result(): TreeMap[K, V] =
      val published = RedBlackTree.beforePublish(tree)
      val entries = emptyBuffer[(K, Any)]
      RedBlackTree.entriesInto(published, entries)
      val out = new TreeMap[K, V](entries, ord)
      out.rb = published
      out

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
