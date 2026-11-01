// scala-library's `Growable`, `Shrinkable` and `Builder` contract for the sorted builders, branch by branch.
// `Growable.addAll`'s fallback walks a source by its iterator, each element through `addOne`, never by a `foreach` of
// the source's own (the immutable `TreeSet` and `TreeMap` builders reach it for any source they do not adopt, the
// mutable `TreeSet`'s builder and the mutable set's `++=` for every source, `mapResult` forwarding to it); a builder
// that is its own source is copied first (`Buffer.from`, `ArrayBuffer.from`'s walk: its size, then its iterator). The
// mutable set's `--=` is `Shrinkable.subtractAll`: itself cleared, a
// linear sequence head by head, anything else by its iterator. `sizeHint(coll, delta)` hints only where the source's
// size is known, never below 0 (an immutable sorted set, a `ListSet` or a `ListMap` with elements knows none), and
// `sizeHintBounded` the smaller of the two; the sorted builders know no size.
// `TreeMapBuilder` has one `adder` for its life, whose accumulator a nested `addAll` of a map from inside a map's
// `foreachEntry` starts again, commits and empties, and which a saved callback still reaches.
import scala.collection.immutable.{TreeMap, TreeSet}
import scala.collection.mutable

class Source[A](xs: List[A], known: Int = -1) extends IterableOnce[A]:
  override def iterator: Iterator[A] = xs.iterator
  override def knownSize: Int = known
  def foreach[U](f: A => U): Unit =
    println("foreach called")
    xs.foreach(f)

class Acc extends mutable.Builder[Int, List[Int]], IterableOnce[Int]:
  private val items = mutable.ArrayBuffer.empty[Int]
  def addOne(elem: Int): this.type =
    items += elem
    this
  def clear(): Unit = items.clear()
  def result(): List[Int] = items.toList
  override def iterator: Iterator[Int] = items.toList.iterator
  def foreach[U](f: Int => U): Unit = items.toList.foreach(f)
  override def knownSize: Int = -1
  override def sizeHint(size: Int): Unit = println(s"hint $size")

class Traced extends mutable.Builder[Int, List[Int]], IterableOnce[Int]:
  private val items = mutable.ArrayBuffer.empty[Int]
  def addOne(elem: Int): this.type =
    items += elem
    this
  def clear(): Unit = items.clear()
  def result(): List[Int] = items.toList
  override def iterator: Iterator[Int] =
    println("iterator")
    items.toList.iterator
  def foreach[U](f: Int => U): Unit =
    println("foreach")
    items.toList.foreach(f)
  override def knownSize: Int =
    println("knownSize")
    -1

object Main:
  def main(args: Array[String]): Unit =
    val s = TreeSet.newBuilder[Int]
    s ++= new Source(List(2, 1))
    s ++= TreeSet(4, 3)(using Ordering.Int.reverse)
    s ++= mutable.TreeSet(5)
    println(s"${s.result()} ${s.knownSize}")
    val m = TreeMap.newBuilder[Int, String]
    m ++= new Source(List(2 -> "b", 1 -> "a"))
    println(s"${m.result()} ${m.knownSize}")
    val u = mutable.TreeSet.newBuilder[Int]
    u ++= new Source(List(2, 1))
    println(s"${u.result()} ${u.knownSize}")
    val r = TreeSet.newBuilder[Int].mapResult(_.toList)
    r ++= new Source(List(3, 1))
    println(s"${r.result()} ${r.knownSize}")

    val acc = new Acc
    acc += 1
    acc += 2
    acc ++= acc
    println(acc.result())
    acc.sizeHint(new Source(List(1, 2, 3), 3), 2)
    acc.sizeHint(Set(1), -5)
    acc.sizeHint(new Source(List(1, 2)), 1)
    acc.sizeHintBounded(10, Set(1, 2, 3))
    acc.sizeHintBounded(2, Set(1, 2, 3))
    acc.sizeHintBounded(2, List(1, 2, 3))
    acc.mapResult(_.size).sizeHint(mutable.TreeSet(1, 2), 0)
    acc.sizeHint(TreeSet(1, 2))
    acc.sizeHint(scala.collection.immutable.SortedSet(1, 2, 3), 1)
    acc.sizeHint(scala.collection.immutable.ListSet(1, 2))
    acc.sizeHint(scala.collection.immutable.ListMap(1 -> 1))
    acc.sizeHint(scala.collection.immutable.ListSet.empty[Int], 1)
    val traced = new Traced
    traced += 1
    traced ++= traced
    println(traced.result())

    val set = mutable.TreeSet(3, 1)
    set ++= new Source(List(4, 2))
    set ++= set
    println(s"$set ${set.knownSize}")
    set --= new Source(List(4))
    set --= List(1, 9)
    set.subtractAll(Vector(2))
    println(set)
    set --= set
    println(s"$set ${set.knownSize}")

    // One adder: the nested `addAll` restarts the accumulator from the builder's tree and commits it; the outer walk
    // goes on in the emptied accumulator, which its own commit then makes the tree.
    val b = TreeMap.newBuilder[Int, String]
    val inner = mutable.HashMap(2 -> "b")
    val outer = new mutable.HashMap[Int, String]:
      override def foreachEntry[U](f: (Int, String) => U): Unit =
        f(1, "a")
        b.addAll(inner)
        f(3, "c")
    b.addAll(outer)
    println(b.result())
    // `clear` inside the walk empties the tree, not the accumulator.
    val c = TreeMap.newBuilder[Int, String]
    c += 0 -> "z"
    val clearing = new mutable.HashMap[Int, String]:
      override def foreachEntry[U](f: (Int, String) => U): Unit =
        f(1, "a")
        c.clear()
        f(3, "c")
    c.addAll(clearing)
    println(c.result())
    // A callback kept past its walk is still the builder's one adder: called later it adds to an accumulator no tree
    // reads, and inside a later walk to that walk's.
    var saved: (Int, String) => Any = null
    val d = TreeMap.newBuilder[Int, String]
    val keeping = new mutable.HashMap[Int, String]:
      override def foreachEntry[U](f: (Int, String) => U): Unit =
        saved = f
        f(1, "a")
    d.addAll(keeping)
    saved(5, "e")
    println(d.result())
    val later = new mutable.HashMap[Int, String]:
      override def foreachEntry[U](f: (Int, String) => U): Unit =
        println(s"same adder ${f.asInstanceOf[AnyRef] eq saved.asInstanceOf[AnyRef]}")
        f(2, "b")
        saved(6, "f")
    d.addAll(later)
    println(d.result())
