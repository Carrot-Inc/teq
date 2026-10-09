// `java.util`'s sorted maps and sets for JavaScript and the interpreter, written from the JDK's
// documented behaviour: `TreeMap` over a red-black tree of its own nodes, the views of a map (its
// sub-maps, its descending map, its key, value and entry sets) over the same tree, and `TreeSet`
// over a `TreeMap`, as the JDK has them; on the JVM the classes are the JDK's own (`@jvmClass`).
//
// An iterator does not fail fast: when the map changes other than through it, it goes on from
// the key it gave last, in the map as it is then.
package java.util:

  @jvmClass("java/util/SortedMap")
  trait SortedMap[K, V] extends Map[K, V]:
    def comparator(): Comparator[? >: K]
    def firstKey(): K
    def lastKey(): K
    def headMap(toKey: K): SortedMap[K, V]
    def tailMap(fromKey: K): SortedMap[K, V]
    def subMap(fromKey: K, toKey: K): SortedMap[K, V]

  @jvmClass("java/util/NavigableMap")
  trait NavigableMap[K, V] extends SortedMap[K, V]:
    def lowerEntry(key: K): Map.Entry[K, V]
    def lowerKey(key: K): K
    def floorEntry(key: K): Map.Entry[K, V]
    def floorKey(key: K): K
    def ceilingEntry(key: K): Map.Entry[K, V]
    def ceilingKey(key: K): K
    def higherEntry(key: K): Map.Entry[K, V]
    def higherKey(key: K): K
    def firstEntry(): Map.Entry[K, V]
    def lastEntry(): Map.Entry[K, V]
    def pollFirstEntry(): Map.Entry[K, V]
    def pollLastEntry(): Map.Entry[K, V]
    def descendingMap(): NavigableMap[K, V]
    def navigableKeySet(): NavigableSet[K]
    def descendingKeySet(): NavigableSet[K]
    def subMap(fromKey: K, fromInclusive: Boolean, toKey: K, toInclusive: Boolean): NavigableMap[K, V]
    def headMap(toKey: K, inclusive: Boolean): NavigableMap[K, V]
    def tailMap(fromKey: K, inclusive: Boolean): NavigableMap[K, V]
    // Declared again beside their overloads, as the JDK's interface has them.
    def subMap(fromKey: K, toKey: K): SortedMap[K, V]
    def headMap(toKey: K): SortedMap[K, V]
    def tailMap(fromKey: K): SortedMap[K, V]

  @jvmClass("java/util/SortedSet")
  trait SortedSet[E] extends Set[E]:
    def comparator(): Comparator[? >: E]
    def first(): E
    def last(): E
    def headSet(toElement: E): SortedSet[E]
    def tailSet(fromElement: E): SortedSet[E]
    def subSet(fromElement: E, toElement: E): SortedSet[E]

  @jvmClass("java/util/NavigableSet")
  trait NavigableSet[E] extends SortedSet[E]:
    def lower(e: E): E
    def floor(e: E): E
    def ceiling(e: E): E
    def higher(e: E): E
    def pollFirst(): E
    def pollLast(): E
    def descendingSet(): NavigableSet[E]
    def descendingIterator(): Iterator[E]
    def subSet(fromElement: E, fromInclusive: Boolean, toElement: E, toInclusive: Boolean): NavigableSet[E]
    def headSet(toElement: E, inclusive: Boolean): NavigableSet[E]
    def tailSet(fromElement: E, inclusive: Boolean): NavigableSet[E]
    def subSet(fromElement: E, toElement: E): SortedSet[E]
    def headSet(toElement: E): SortedSet[E]
    def tailSet(fromElement: E): SortedSet[E]

  @jvmClass("java/util/TreeMap")
  class TreeMap[K, V](cmp: Comparator[? >: K]) extends AbstractMap[K, V], NavigableMap[K, V]:
    def this() = this(null: Comparator[? >: K])
    // Natural ordering, whatever the map's own (the JDK's `TreeMap(Map)`).
    def this(m: Map[? <: K, ? <: V]) =
      this(null: Comparator[? >: K])
      putAll(m)
    def this(m: SortedMap[K, ? <: V]) =
      this(m.comparator())
      buildFromSorted(m.size(), m.entrySet().iterator(), null)

    private[util] var root: TreeNode[K, V] = null
    private var count = 0
    // Counts the insertions and removals, by which an iterator knows the map changed under it.
    private[util] var modCount = 0
    private var whole: SortedView[K, V] = null

    // The JDK's order of two keys: the comparator's, or the first key's `compareTo`.
    private[util] def compare(a: Any, b: Any): Int =
      if cmp == null then SortedOps.compareNatural(a, b)
      else cmp.asInstanceOf[Comparator[Any]].compare(a, b)

    // The whole map as a view, ascending: what the key, value and entry sets and the sub-maps of
    // the map are made from.
    private[util] def all: SortedView[K, V] =
      if whole == null then whole = new SortedView(this, true, null.asInstanceOf[K], true, true, null.asInstanceOf[K], true, false)
      whole
    // Whether a view is that one: the map's own key, value and entry sets, not a view's.
    private[util] def isWhole(view: SortedView[K, V]): Boolean = view eq whole

    // The JDK's `putAll`: into an empty map, a sorted map of the same order is taken in linear
    // time (`buildFromSorted`), its keys never compared.
    override def putAll(m: Map[? <: K, ? <: V]): Unit =
      val n = m.size()
      m match
        case sorted: SortedMap[?, ?] if count == 0 && n != 0 && Objects.equals(cmp, sorted.comparator()) =>
          modCount += 1
          buildFromSorted(n, sorted.entrySet().iterator(), null)
        case _ => super.putAll(m)

    // The JDK's `buildFromSorted`: a balanced tree of the `n` entries, or with `present` keys, the
    // iterator gives in order, the nodes of an incomplete bottom level red; no key is compared.
    private[util] def buildFromSorted(n: Int, it: Iterator[?], present: AnyRef): Unit =
      count = n
      root = build(0, 0, n - 1, 31 - Integer.numberOfLeadingZeros(n + 1), it.asInstanceOf[Iterator[Any]], present)
    private def build(level: Int, lo: Int, hi: Int, redLevel: Int, it: Iterator[Any], present: AnyRef): TreeNode[K, V] =
      if hi < lo then null
      else
        val mid = (lo + hi) >>> 1
        val left = if lo < mid then build(level + 1, lo, mid - 1, redLevel, it, present) else null
        val middle =
          if present == null then
            val e = it.next().asInstanceOf[Map.Entry[K, V]]
            new TreeNode[K, V](e.getKey, e.getValue, null)
          else new TreeNode[K, V](it.next().asInstanceOf[K], present.asInstanceOf[V], null)
        middle.black = level != redLevel
        if left != null then
          middle.left = left
          left.parent = middle
        if mid < hi then
          val right = build(level + 1, mid + 1, hi, redLevel, it, present)
          middle.right = right
          right.parent = middle
        middle

    def comparator(): Comparator[? >: K] = cmp
    def size(): Int = count
    override def isEmpty(): Boolean = count == 0
    def get(key: Any): V =
      val n = node(key)
      if n == null then null.asInstanceOf[V] else n.value
    def containsKey(key: Any): Boolean = node(key) != null
    def put(key: K, value: V): V =
      var parent: TreeNode[K, V] = null
      var n = root
      var c = 0
      // The JDK compares the first key with itself, which checks it as any later key is.
      if n == null then compare(key, key)
      while n != null do
        parent = n
        c = compare(key, n.key)
        if c < 0 then n = n.left
        else if c > 0 then n = n.right
        else
          val old = n.value
          n.value = value
          return old
      val added = new TreeNode[K, V](key, value, parent)
      if parent == null then root = added
      else if c < 0 then parent.left = added
      else parent.right = added
      count += 1
      modCount += 1
      balanceInserted(added)
      null.asInstanceOf[V]
    def remove(key: Any): V =
      val n = node(key)
      if n == null then null.asInstanceOf[V]
      else
        unlink(n)
        n.value
    def clear(): Unit =
      root = null
      count = 0
      modCount += 1

    def firstKey(): K = SortedOps.keyOrThrow(firstNode)
    def lastKey(): K = SortedOps.keyOrThrow(lastNode)
    def firstEntry(): Map.Entry[K, V] = SortedOps.snapshot(firstNode)
    def lastEntry(): Map.Entry[K, V] = SortedOps.snapshot(lastNode)
    def pollFirstEntry(): Map.Entry[K, V] = poll(firstNode)
    def pollLastEntry(): Map.Entry[K, V] = poll(lastNode)
    def lowerEntry(key: K): Map.Entry[K, V] = SortedOps.snapshot(below(key, false))
    def lowerKey(key: K): K = SortedOps.keyOrNull(below(key, false))
    def floorEntry(key: K): Map.Entry[K, V] = SortedOps.snapshot(below(key, true))
    def floorKey(key: K): K = SortedOps.keyOrNull(below(key, true))
    def ceilingEntry(key: K): Map.Entry[K, V] = SortedOps.snapshot(above(key, true))
    def ceilingKey(key: K): K = SortedOps.keyOrNull(above(key, true))
    def higherEntry(key: K): Map.Entry[K, V] = SortedOps.snapshot(above(key, false))
    def higherKey(key: K): K = SortedOps.keyOrNull(above(key, false))

    def keySet(): Set[K] = all.navigableKeySet()
    def navigableKeySet(): NavigableSet[K] = all.navigableKeySet()
    def descendingKeySet(): NavigableSet[K] = all.descendingKeySet()
    def values(): Collection[V] = all.values()
    def entrySet(): Set[Map.Entry[K, V]] = all.entrySet()
    def descendingMap(): NavigableMap[K, V] = all.descendingMap()
    def subMap(fromKey: K, fromInclusive: Boolean, toKey: K, toInclusive: Boolean): NavigableMap[K, V] =
      all.subMap(fromKey, fromInclusive, toKey, toInclusive)
    def headMap(toKey: K, inclusive: Boolean): NavigableMap[K, V] = all.headMap(toKey, inclusive)
    def tailMap(fromKey: K, inclusive: Boolean): NavigableMap[K, V] = all.tailMap(fromKey, inclusive)
    def subMap(fromKey: K, toKey: K): SortedMap[K, V] = all.subMap(fromKey, true, toKey, false)
    def headMap(toKey: K): SortedMap[K, V] = all.headMap(toKey, false)
    def tailMap(fromKey: K): SortedMap[K, V] = all.tailMap(fromKey, true)
    override def toString: String = SortedOps.mapString(this)

    // The node of a key, null when the map has none. Under natural ordering a null key or one
    // that is not `Comparable` fails before the search, on an empty map too, as the JDK's lookup.
    private[util] def node(key: Any): TreeNode[K, V] =
      if cmp == null then SortedOps.natural(key)
      var n = root
      while n != null do
        val c = compare(key, n.key)
        if c < 0 then n = n.left
        else if c > 0 then n = n.right
        else return n
      null
    // The node of the least key above `key`, or at it when `inclusive`; null when none.
    private[util] def above(key: Any, inclusive: Boolean): TreeNode[K, V] =
      var n = root
      var best: TreeNode[K, V] = null
      while n != null do
        val c = compare(key, n.key)
        if c < 0 then
          best = n
          n = n.left
        else if c == 0 && inclusive then return n
        else n = n.right
      best
    // The node of the greatest key below `key`, or at it when `inclusive`; null when none.
    private[util] def below(key: Any, inclusive: Boolean): TreeNode[K, V] =
      var n = root
      var best: TreeNode[K, V] = null
      while n != null do
        val c = compare(key, n.key)
        if c > 0 then
          best = n
          n = n.right
        else if c == 0 && inclusive then return n
        else n = n.left
      best
    private[util] def firstNode: TreeNode[K, V] = if root == null then null else SortedOps.leftmost(root)
    private[util] def lastNode: TreeNode[K, V] = if root == null then null else SortedOps.rightmost(root)
    // The snapshot of a node, which is taken out of the map.
    private[util] def poll(n: TreeNode[K, V]): Map.Entry[K, V] =
      val e = SortedOps.snapshot(n)
      if n != null then unlink(n)
      e

    // The red-black tree, as Cormen, Leiserson, Rivest and Stein give it. A removal relinks the
    // nodes rather than moving keys between them, so that a node stays its mapping's entry for
    // as long as the mapping is in the map; a removed node keeps its key and value.
    private def red(n: TreeNode[K, V]): Boolean = n != null && !n.black
    private def relink(n: TreeNode[K, V], by: TreeNode[K, V]): Unit =
      val p = n.parent
      if p == null then root = by
      else if n eq p.left then p.left = by
      else p.right = by
      if by != null then by.parent = p
    private def rotateLeft(x: TreeNode[K, V]): Unit =
      val y = x.right
      x.right = y.left
      if y.left != null then y.left.parent = x
      relink(x, y)
      y.left = x
      x.parent = y
    private def rotateRight(x: TreeNode[K, V]): Unit =
      val y = x.left
      x.left = y.right
      if y.right != null then y.right.parent = x
      relink(x, y)
      y.right = x
      x.parent = y
    private def balanceInserted(added: TreeNode[K, V]): Unit =
      var z = added
      while red(z.parent) do
        val p = z.parent
        val g = p.parent
        if p eq g.left then
          val u = g.right
          if red(u) then
            p.black = true
            u.black = true
            g.black = false
            z = g
          else
            if z eq p.right then
              z = p
              rotateLeft(z)
            z.parent.black = true
            z.parent.parent.black = false
            rotateRight(z.parent.parent)
        else
          val u = g.left
          if red(u) then
            p.black = true
            u.black = true
            g.black = false
            z = g
          else
            if z eq p.left then
              z = p
              rotateRight(z)
            z.parent.black = true
            z.parent.parent.black = false
            rotateLeft(z.parent.parent)
      root.black = true
    private[util] def unlink(z: TreeNode[K, V]): Unit =
      count -= 1
      modCount += 1
      // `x` takes the place of the node that leaves its position, under `parent`.
      var x: TreeNode[K, V] = null
      var parent: TreeNode[K, V] = null
      var removedBlack = z.black
      if z.left == null then
        x = z.right
        parent = z.parent
        relink(z, z.right)
      else if z.right == null then
        x = z.left
        parent = z.parent
        relink(z, z.left)
      else
        val y = SortedOps.leftmost(z.right)
        removedBlack = y.black
        x = y.right
        if y.parent eq z then parent = y
        else
          parent = y.parent
          relink(y, y.right)
          y.right = z.right
          y.right.parent = y
        relink(z, y)
        y.left = z.left
        y.left.parent = y
        y.black = z.black
      if removedBlack then balanceRemoved(x, parent)
    private def balanceRemoved(start: TreeNode[K, V], under: TreeNode[K, V]): Unit =
      var x = start
      var parent = under
      while (x ne root) && !red(x) do
        if x eq parent.left then
          var w = parent.right
          if red(w) then
            w.black = true
            parent.black = false
            rotateLeft(parent)
            w = parent.right
          if !red(w.left) && !red(w.right) then
            w.black = false
            x = parent
            parent = x.parent
          else
            if !red(w.right) then
              w.left.black = true
              w.black = false
              rotateRight(w)
              w = parent.right
            w.black = parent.black
            parent.black = true
            w.right.black = true
            rotateLeft(parent)
            x = root
            parent = null
        else
          var w = parent.left
          if red(w) then
            w.black = true
            parent.black = false
            rotateRight(parent)
            w = parent.left
          if !red(w.left) && !red(w.right) then
            w.black = false
            x = parent
            parent = x.parent
          else
            if !red(w.left) then
              w.right.black = true
              w.black = false
              rotateLeft(w)
              w = parent.left
            w.black = parent.black
            parent.black = true
            w.left.black = true
            rotateRight(parent)
            x = root
            parent = null
      if x != null then x.black = true

  // A mapping of a tree map and its node in the tree: the entry the entry set gives, whose
  // `setValue` writes through.
  private final class TreeNode[K, V](val key: K, var value: V, var parent: TreeNode[K, V]) extends Map.Entry[K, V]:
    var left: TreeNode[K, V] = null
    var right: TreeNode[K, V] = null
    var black: Boolean = false
    def getKey: K = key
    def getValue: V = value
    def setValue(v: V): V =
      val old = value
      value = v
      old
    override def equals(o: Any): Boolean = SortedOps.entryEquals(this, o)
    override def hashCode: Int = SortedOps.entryHash(this)
    override def toString: String = "" + key + "=" + value

  // A tree map between optional bounds (`fromStart` and `toEnd` when unbounded), ascending or
  // descending: the map's sub-maps and descending maps, and the whole map, unbounded and
  // ascending, behind its key, value and entry sets. Every member reads and writes the map; a key
  // out of the bounds is absent to a lookup or a removal and refused by `put`.
  private final class SortedView[K, V](val tree: TreeMap[K, V], fromStart: Boolean, lo: K, loInclusive: Boolean,
      toEnd: Boolean, hi: K, hiInclusive: Boolean, val descending: Boolean) extends AbstractMap[K, V], NavigableMap[K, V]:
    // The JDK's checks of the bounds, which fail for a key the map cannot compare.
    if !fromStart && !toEnd then
      if tree.compare(lo, hi) > 0 then throw new IllegalArgumentException("fromKey > toKey")
    else
      if !fromStart then tree.compare(lo, lo)
      if !toEnd then tree.compare(hi, hi)

    private var sized = -1
    private var sizedAt = 0
    // The views of the view, made once as the JDK's are.
    private var keyView: SortedKeys[K, V] = null
    private var valueView: SortedValues[K, V] = null
    private var entryView: SortedEntries[K, V] = null
    private var reverseView: SortedView[K, V] = null

    private[util] def tooLow(key: Any): Boolean =
      !fromStart && {
        val c = tree.compare(key, lo)
        c < 0 || c == 0 && !loInclusive
      }
    private[util] def tooHigh(key: Any): Boolean =
      !toEnd && {
        val c = tree.compare(key, hi)
        c > 0 || c == 0 && !hiInclusive
      }
    private[util] def inRange(key: Any): Boolean = !tooLow(key) && !tooHigh(key)
    // A new bound inside these: within them, or at an exclusive one when the new bound is
    // exclusive too.
    private def inRange(key: Any, inclusive: Boolean): Boolean =
      if inclusive then inRange(key)
      else (fromStart || tree.compare(key, lo) >= 0) && (toEnd || tree.compare(hi, key) >= 0)

    // The range's nodes in the tree's ascending order, whatever the view's direction.
    private[util] def lowest: TreeNode[K, V] =
      val n = if fromStart then tree.firstNode else tree.above(lo, loInclusive)
      if n == null || tooHigh(n.key) then null else n
    private[util] def highest: TreeNode[K, V] =
      val n = if toEnd then tree.lastNode else tree.below(hi, hiInclusive)
      if n == null || tooLow(n.key) then null else n
    private[util] def above(key: Any, inclusive: Boolean): TreeNode[K, V] =
      if tooLow(key) then lowest
      else
        val n = tree.above(key, inclusive)
        if n == null || tooHigh(n.key) then null else n
    private[util] def below(key: Any, inclusive: Boolean): TreeNode[K, V] =
      if tooHigh(key) then highest
      else
        val n = tree.below(key, inclusive)
        if n == null || tooLow(n.key) then null else n
    // The same in the view's own order.
    private def first: TreeNode[K, V] = if descending then highest else lowest
    private def last: TreeNode[K, V] = if descending then lowest else highest
    private def after(key: Any, inclusive: Boolean): TreeNode[K, V] = if descending then below(key, inclusive) else above(key, inclusive)
    private def before(key: Any, inclusive: Boolean): TreeNode[K, V] = if descending then above(key, inclusive) else below(key, inclusive)

    private def unbounded: Boolean = fromStart && toEnd
    def comparator(): Comparator[? >: K] =
      if descending then new ReverseComparator[K](tree.comparator().asInstanceOf[Comparator[K]]) else tree.comparator()
    // The JDK counts a bounded range's mappings, again only once the map has changed.
    def size(): Int =
      if unbounded then tree.size()
      else
        if sized < 0 || sizedAt != tree.modCount then
          var k = 0
          val it = walk[TreeNode[K, V]](0, false)
          while it.hasNext do
            it.next()
            k += 1
          sized = k
          sizedAt = tree.modCount
        sized
    override def isEmpty(): Boolean = if unbounded then tree.isEmpty() else lowest == null
    def get(key: Any): V = if inRange(key) then tree.get(key) else null.asInstanceOf[V]
    def containsKey(key: Any): Boolean = inRange(key) && tree.containsKey(key)
    def put(key: K, value: V): V =
      if !inRange(key) then throw new IllegalArgumentException("key out of range")
      tree.put(key, value)
    def remove(key: Any): V = if inRange(key) then tree.remove(key) else null.asInstanceOf[V]
    def clear(): Unit =
      if unbounded then tree.clear()
      else
        val it = walk[TreeNode[K, V]](0, false)
        while it.hasNext do
          it.next()
          it.remove()

    def firstKey(): K = SortedOps.keyOrThrow(first)
    def lastKey(): K = SortedOps.keyOrThrow(last)
    def firstEntry(): Map.Entry[K, V] = SortedOps.snapshot(first)
    def lastEntry(): Map.Entry[K, V] = SortedOps.snapshot(last)
    def pollFirstEntry(): Map.Entry[K, V] = tree.poll(first)
    def pollLastEntry(): Map.Entry[K, V] = tree.poll(last)
    def lowerEntry(key: K): Map.Entry[K, V] = SortedOps.snapshot(before(key, false))
    def lowerKey(key: K): K = SortedOps.keyOrNull(before(key, false))
    def floorEntry(key: K): Map.Entry[K, V] = SortedOps.snapshot(before(key, true))
    def floorKey(key: K): K = SortedOps.keyOrNull(before(key, true))
    def ceilingEntry(key: K): Map.Entry[K, V] = SortedOps.snapshot(after(key, true))
    def ceilingKey(key: K): K = SortedOps.keyOrNull(after(key, true))
    def higherEntry(key: K): Map.Entry[K, V] = SortedOps.snapshot(after(key, false))
    def higherKey(key: K): K = SortedOps.keyOrNull(after(key, false))

    def keySet(): Set[K] = navigableKeySet()
    def navigableKeySet(): NavigableSet[K] =
      if keyView == null then keyView = new SortedKeys(this)
      keyView
    def descendingKeySet(): NavigableSet[K] = reversed.navigableKeySet()
    def values(): Collection[V] =
      if valueView == null then valueView = new SortedValues(this, tree.isWhole(this))
      valueView
    def entrySet(): Set[Map.Entry[K, V]] =
      if entryView == null then entryView = new SortedEntries(this)
      entryView
    def descendingMap(): NavigableMap[K, V] = reversed
    def subMap(fromKey: K, fromInclusive: Boolean, toKey: K, toInclusive: Boolean): NavigableMap[K, V] =
      sub(fromKey, fromInclusive, toKey, toInclusive)
    def headMap(toKey: K, inclusive: Boolean): NavigableMap[K, V] = head(toKey, inclusive)
    def tailMap(fromKey: K, inclusive: Boolean): NavigableMap[K, V] = tail(fromKey, inclusive)
    def subMap(fromKey: K, toKey: K): SortedMap[K, V] = sub(fromKey, true, toKey, false)
    def headMap(toKey: K): SortedMap[K, V] = head(toKey, false)
    def tailMap(fromKey: K): SortedMap[K, V] = tail(fromKey, true)
    override def toString: String = SortedOps.mapString(this)

    // The views of the view, bounds given in its own order.
    private[util] def reversed: SortedView[K, V] =
      if reverseView == null then reverseView = new SortedView(tree, fromStart, lo, loInclusive, toEnd, hi, hiInclusive, !descending)
      reverseView
    private[util] def sub(fromKey: K, fromInclusive: Boolean, toKey: K, toInclusive: Boolean): SortedView[K, V] =
      if !inRange(fromKey, fromInclusive) then throw new IllegalArgumentException("fromKey out of range")
      if !inRange(toKey, toInclusive) then throw new IllegalArgumentException("toKey out of range")
      if descending then new SortedView(tree, false, toKey, toInclusive, false, fromKey, fromInclusive, true)
      else new SortedView(tree, false, fromKey, fromInclusive, false, toKey, toInclusive, false)
    private[util] def head(toKey: K, inclusive: Boolean): SortedView[K, V] =
      if !inRange(toKey, inclusive) then throw new IllegalArgumentException("toKey out of range")
      if descending then new SortedView(tree, false, toKey, inclusive, toEnd, hi, hiInclusive, true)
      else new SortedView(tree, fromStart, lo, loInclusive, false, toKey, inclusive, false)
    private[util] def tail(fromKey: K, inclusive: Boolean): SortedView[K, V] =
      if !inRange(fromKey, inclusive) then throw new IllegalArgumentException("fromKey out of range")
      if descending then new SortedView(tree, fromStart, lo, loInclusive, false, fromKey, inclusive, true)
      else new SortedView(tree, false, fromKey, inclusive, toEnd, hi, hiInclusive, false)

    // The range's nodes as entries (0), keys (1) or values (2), in the view's order or against it.
    private[util] def walk[E](kind: Int, against: Boolean): Iterator[E] =
      new TreeIterator[K, V, E](this, kind, descending != against)
    // The node of a mapping equal to `o`, an entry, in the range.
    private[util] def mapping(o: Any): TreeNode[K, V] = o match
      case e: Map.Entry[?, ?] =>
        if !inRange(e.getKey) then null
        else
          val n = tree.node(e.getKey)
          if n != null && AbstractMap.valueEquals(n.value, e.getValue) then n else null
      case _ => null
    // Whether the range had a mapping of `key`, which is removed.
    private[util] def removeKey(key: Any): Boolean =
      if !inRange(key) then false
      else
        val n = tree.node(key)
        if n != null then tree.unlink(n)
        n != null

  // The nodes of a view's range in one direction, as entries, keys or values. When the map
  // changed other than through the iterator, the next node is looked up again after the key
  // given last, and `remove` removes that key's mapping if the map still has one: the node given
  // last may have left the tree.
  private final class TreeIterator[K, V, E](view: SortedView[K, V], kind: Int, descending: Boolean) extends Iterator[E]:
    private val tree = view.tree
    private var upcoming: TreeNode[K, V] = if descending then view.highest else view.lowest
    private var last: TreeNode[K, V] = null
    private var lastKey: K = null.asInstanceOf[K]
    private var lastStale = false
    private var started = false
    private var expected = tree.modCount
    private def sync(): Unit =
      if expected != tree.modCount then
        upcoming =
          if !started then (if descending then view.highest else view.lowest)
          else if descending then view.below(lastKey, false)
          else view.above(lastKey, false)
        expected = tree.modCount
        lastStale = true
    def hasNext: Boolean =
      sync()
      upcoming != null
    def next(): E =
      sync()
      val n = upcoming
      if n == null then throw new NoSuchElementException()
      last = n
      lastKey = n.key
      lastStale = false
      started = true
      val s = if descending then SortedOps.predecessor(n) else SortedOps.successor(n)
      upcoming = if s == null || (if descending then view.tooLow(s.key) else view.tooHigh(s.key)) then null else s
      (if kind == 0 then n else if kind == 1 then n.key else n.value).asInstanceOf[E]
    override def remove(): Unit =
      if last == null then throw new IllegalStateException()
      sync()
      val n = if lastStale then tree.node(lastKey) else last
      if n != null then tree.unlink(n)
      expected = tree.modCount
      last = null

  // The entry set of a view: an entry of a mapping in the range is a member, and removing one
  // removes the mapping.
  private final class SortedEntries[K, V](view: SortedView[K, V]) extends AbstractSet[Map.Entry[K, V]]:
    def size(): Int = view.size()
    override def isEmpty(): Boolean = view.isEmpty()
    def iterator(): Iterator[Map.Entry[K, V]] = view.walk[Map.Entry[K, V]](0, false)
    override def contains(o: Any): Boolean = view.mapping(o) != null
    override def remove(o: Any): Boolean =
      val n = view.mapping(o)
      if n != null then view.tree.unlink(n)
      n != null
    override def clear(): Unit = view.clear()
    override def toString: String = SortedOps.collectionString(this)

  // The values of a view, in its order, compared as the JDK's maps compare them (and so
  // `containsValue`): `remove` removes the first mapping of the value. The JDK's `equals` is
  // called on the value searched for, as `AbstractMap.containsValue` and `AbstractCollection.remove`
  // call it, but on the map's value by the remove of the whole map's values (`TreeMap.Values`).
  private final class SortedValues[K, V](view: SortedView[K, V], whole: Boolean) extends Collection[V]:
    def size(): Int = view.size()
    override def isEmpty(): Boolean = view.isEmpty()
    def iterator(): Iterator[V] = view.walk[V](2, false)
    override def contains(o: Any): Boolean =
      val it = iterator()
      var found = false
      while !found && it.hasNext do found = AbstractMap.valueEquals(o, it.next())
      found
    override def remove(o: Any): Boolean =
      val it = iterator()
      var found = false
      while !found && it.hasNext do
        val v = it.next()
        if (if whole then AbstractMap.valueEquals(v, o) else AbstractMap.valueEquals(o, v)) then
          it.remove()
          found = true
      found
    override def clear(): Unit = view.clear()
    override def toString: String = SortedOps.collectionString(this)

  // The keys of a view, a navigable set that removes from the map and adds nothing.
  private final class SortedKeys[K, V](view: SortedView[K, V]) extends AbstractSet[K], NavigableSet[K]:
    def size(): Int = view.size()
    override def isEmpty(): Boolean = view.isEmpty()
    def iterator(): Iterator[K] = view.walk[K](1, false)
    def descendingIterator(): Iterator[K] = view.walk[K](1, true)
    override def contains(o: Any): Boolean = view.containsKey(o)
    override def remove(o: Any): Boolean = view.removeKey(o)
    override def clear(): Unit = view.clear()
    override def toString: String = SortedOps.collectionString(this)
    def comparator(): Comparator[? >: K] = view.comparator()
    def first(): K = view.firstKey()
    def last(): K = view.lastKey()
    def lower(e: K): K = view.lowerKey(e)
    def floor(e: K): K = view.floorKey(e)
    def ceiling(e: K): K = view.ceilingKey(e)
    def higher(e: K): K = view.higherKey(e)
    def pollFirst(): K = SortedOps.entryKey(view.pollFirstEntry())
    def pollLast(): K = SortedOps.entryKey(view.pollLastEntry())
    def descendingSet(): NavigableSet[K] = new SortedKeys(view.reversed)
    def subSet(fromElement: K, fromInclusive: Boolean, toElement: K, toInclusive: Boolean): NavigableSet[K] =
      new SortedKeys(view.sub(fromElement, fromInclusive, toElement, toInclusive))
    def headSet(toElement: K, inclusive: Boolean): NavigableSet[K] = new SortedKeys(view.head(toElement, inclusive))
    def tailSet(fromElement: K, inclusive: Boolean): NavigableSet[K] = new SortedKeys(view.tail(fromElement, inclusive))
    def subSet(fromElement: K, toElement: K): SortedSet[K] = subSet(fromElement, true, toElement, false)
    def headSet(toElement: K): SortedSet[K] = headSet(toElement, false)
    def tailSet(fromElement: K): SortedSet[K] = tailSet(fromElement, true)

  // The keys of a navigable map whose values are all one marker, as the JDK's: its subsets and
  // its descending set are tree sets over the map's views.
  @jvmClass("java/util/TreeSet")
  class TreeSet[E] private (m: NavigableMap[E, AnyRef]) extends AbstractSet[E], NavigableSet[E]:
    def this() = this(new TreeMap[E, AnyRef]())
    def this(comparator: Comparator[? >: E]) = this(new TreeMap[E, AnyRef](comparator))
    def this(c: Collection[? <: E]) =
      this(new TreeMap[E, AnyRef]())
      addAll(c.asInstanceOf[Collection[E]])
    def this(s: SortedSet[E]) =
      this(new TreeMap[E, AnyRef](s.comparator()))
      addAll(s)
    def size(): Int = m.size()
    override def isEmpty(): Boolean = m.isEmpty()
    def iterator(): Iterator[E] = m.navigableKeySet().iterator()
    def descendingIterator(): Iterator[E] = m.descendingKeySet().iterator()
    override def contains(o: Any): Boolean = m.containsKey(o)
    override def add(e: E): Boolean = m.put(e, SortedOps.Present) == null
    // The JDK's `addAll`: into an empty tree set, a sorted set of the same order goes in linear
    // time (`TreeMap.addAllForTreeSet`).
    override def addAll(c: Collection[? <: E]): Boolean =
      (m, c) match
        case (map: TreeMap[?, ?], sorted: SortedSet[?]) if m.size() == 0 && c.size() > 0 && Objects.equals(map.comparator(), sorted.comparator()) =>
          map.modCount += 1
          map.buildFromSorted(c.size(), sorted.iterator(), SortedOps.Present)
          true
        case _ => super.addAll(c)
    override def remove(o: Any): Boolean = m.remove(o) != null
    override def clear(): Unit = m.clear()
    override def toString: String = SortedOps.collectionString(this)
    def comparator(): Comparator[? >: E] = m.comparator()
    def first(): E = m.firstKey()
    def last(): E = m.lastKey()
    def lower(e: E): E = m.lowerKey(e)
    def floor(e: E): E = m.floorKey(e)
    def ceiling(e: E): E = m.ceilingKey(e)
    def higher(e: E): E = m.higherKey(e)
    def pollFirst(): E = SortedOps.entryKey(m.pollFirstEntry())
    def pollLast(): E = SortedOps.entryKey(m.pollLastEntry())
    def descendingSet(): NavigableSet[E] = new TreeSet(m.descendingMap())
    def subSet(fromElement: E, fromInclusive: Boolean, toElement: E, toInclusive: Boolean): NavigableSet[E] =
      new TreeSet(m.subMap(fromElement, fromInclusive, toElement, toInclusive))
    def headSet(toElement: E, inclusive: Boolean): NavigableSet[E] = new TreeSet(m.headMap(toElement, inclusive))
    def tailSet(fromElement: E, inclusive: Boolean): NavigableSet[E] = new TreeSet(m.tailMap(fromElement, inclusive))
    def subSet(fromElement: E, toElement: E): SortedSet[E] = subSet(fromElement, true, toElement, false)
    def headSet(toElement: E): SortedSet[E] = headSet(toElement, false)
    def tailSet(fromElement: E): SortedSet[E] = tailSet(fromElement, true)

  // The comparator of a descending view: the map's reversed, natural ordering reversed without one.
  private final class ReverseComparator[T](cmp: Comparator[T]) extends Comparator[T]:
    def compare(a: T, b: T): Int = if cmp == null then SortedOps.compareNatural(b, a) else cmp.compare(b, a)

  private[java] object SortedOps:
    // The value of every key of a tree set's map.
    val Present: AnyRef = java.lang.Boolean.valueOf(true)

    // A key's natural ordering, the JDK's cast to `Comparable`: a null key fails with a
    // `NullPointerException` and one that is not `Comparable` with a `ClassCastException`.
    def natural(key: Any): Comparable[Any] =
      if key == null then throw new NullPointerException()
      key match
        case c: Comparable[?] => c.asInstanceOf[Comparable[Any]]
        case _ => throw new ClassCastException("the key is not a java.lang.Comparable")
    // `a.compareTo(b)` under natural ordering, failing as `natural(a)` does, and where the JDK's
    // `compareTo` of a string, a box or a boolean fails for its argument: with a
    // `NullPointerException` for a null `b`, with a `ClassCastException` for a `b` of another kind
    // (a string and a number never compare). The numbers other than `Long` are one kind, which
    // JavaScript does not tell apart.
    def compareNatural(a: Any, b: Any): Int =
      val alike = a match
        case null => throw new NullPointerException()
        case _: String => b.isInstanceOf[String]
        case _: Long => b.isInstanceOf[Long]
        case _: Int | _: Double | _: Float | _: Short | _: Byte => isNumber(b)
        case _: Char => b.isInstanceOf[Char]
        case _: Boolean => b.isInstanceOf[Boolean]
        case _: Comparable[?] => true
        case _ => throw new ClassCastException("the key is not a java.lang.Comparable")
      if !alike then
        if b == null then throw new NullPointerException()
        throw new ClassCastException("the keys are not comparable with each other")
      a.asInstanceOf[Comparable[Any]].compareTo(b)
    private def isNumber(x: Any): Boolean = x match
      case _: Int | _: Double | _: Float | _: Short | _: Byte => true
      case _ => false

    def leftmost[K, V](from: TreeNode[K, V]): TreeNode[K, V] =
      var n = from
      while n.left != null do n = n.left
      n
    def rightmost[K, V](from: TreeNode[K, V]): TreeNode[K, V] =
      var n = from
      while n.right != null do n = n.right
      n
    def successor[K, V](n: TreeNode[K, V]): TreeNode[K, V] =
      if n.right != null then leftmost(n.right)
      else
        var c = n
        var p = n.parent
        while p != null && (c eq p.right) do
          c = p
          p = p.parent
        p
    def predecessor[K, V](n: TreeNode[K, V]): TreeNode[K, V] =
      if n.left != null then rightmost(n.left)
      else
        var c = n
        var p = n.parent
        while p != null && (c eq p.left) do
          c = p
          p = p.parent
        p

    def keyOrNull[K, V](n: TreeNode[K, V]): K = if n == null then null.asInstanceOf[K] else n.key
    def keyOrThrow[K, V](n: TreeNode[K, V]): K = if n == null then throw new NoSuchElementException() else n.key
    def entryKey[K, V](e: Map.Entry[K, V]): K = if e == null then null.asInstanceOf[K] else e.getKey
    // What the navigation hands out: a copy of the mapping, which `setValue` cannot change.
    def snapshot[K, V](n: TreeNode[K, V]): Map.Entry[K, V] =
      if n == null then null else new AbstractMap.SimpleImmutableEntry[K, V](n.key, n.value)
    def entryEquals(e: Map.Entry[?, ?], o: Any): Boolean = o match
      case x: Map.Entry[?, ?] => AbstractMap.valueEquals(e.getKey, x.getKey) && AbstractMap.valueEquals(e.getValue, x.getValue)
      case _ => false
    def entryHash(e: Map.Entry[?, ?]): Int = Objects.hashCode(e.getKey) ^ Objects.hashCode(e.getValue)
    // The JDK's `AbstractMap.toString` and `AbstractCollection.toString`, a member that is the
    // map or the collection itself written as such.
    def mapString(m: Map[?, ?]): String =
      val sb = new java.lang.StringBuilder("{")
      val it = m.entrySet().iterator()
      var first = true
      while it.hasNext do
        if !first then sb.append(", ")
        first = false
        val e = it.next()
        sb.append(itself(e.getKey, m, "(this Map)")).append("=").append(itself(e.getValue, m, "(this Map)"))
      sb.append("}").toString
    def collectionString(c: Collection[?]): String =
      val sb = new java.lang.StringBuilder("[")
      val it = c.iterator()
      var first = true
      while it.hasNext do
        if !first then sb.append(", ")
        first = false
        sb.append(itself(it.next(), c, "(this Collection)"))
      sb.append("]").toString
    private def itself(x: Any, owner: AnyRef, text: String): Any = if x.asInstanceOf[AnyRef] eq owner then text else x

