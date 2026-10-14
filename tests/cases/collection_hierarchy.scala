// The collections' common ancestry as scala-library declares it: a program names `scala.collection.Seq`,
// `Iterable`, `IndexedSeq`, `IterableOps` and `SeqOps` (and `scala.collection.immutable.List`) as a library
// does; every map, mutable or sorted, is a `collection.Map`, equal to a map of the same entries whatever its
// kind and hashing alike, and a `PartialFunction` where a function is expected, as a sequence and a set are
// functions; `TreeMap.from` and `TreeSet.from` of a sorted collection of the same ordering give it back,
// comparing nothing.
import scala.collection.immutable.{TreeMap, TreeSet}
import scala.collection.mutable

object Main:
  var calls = 0
  given Ordering[Int] with
    def compare(a: Int, b: Int): Int =
      calls += 1
      java.lang.Integer.compare(a, b)

  def size[A, C](a: scala.collection.IterableOps[A, List, C]) = a.size
  def second[A, C](a: scala.collection.SeqOps[A, Seq, C]): A = a(1)
  def run(f: Int => Int): Int = f(1)
  def test(f: Int => Boolean): Boolean = f(2)

  def main(args: Array[String]): Unit =
    val a: scala.collection.Seq[Int] = List(1)
    val b: scala.collection.Iterable[Int] = a
    val c: scala.collection.IndexedSeq[Int] = Vector(2)
    val d: scala.collection.immutable.List[Int] = List(3)
    val e: scala.collection.immutable.Seq[Int] = d
    println(a.sum + b.sum + c.sum + d.sum + e.sum)
    println(List(size(List(1, 2)), second(List(4, 5))))
    val mm = mutable.Map(1 -> 42)
    val hm = mutable.HashMap(1 -> 43)
    val sm = TreeMap(1 -> 44)
    println(List(run(mm), run(hm), run(sm), run(Map(1 -> 45)), run(Vector(46, 47)), run(mutable.ArrayBuffer(48, 49))))
    println(List(test(Set(2)), test(mutable.Set(3)), test(TreeSet(2))))
    val tm = TreeMap(1 -> "a", 2 -> "b", 3 -> "c")
    val im = Map(1 -> "a", 2 -> "b", 3 -> "c")
    val mu = mutable.HashMap(1 -> "a", 2 -> "b", 3 -> "c")
    println(List(tm == im, im == tm, tm == mu, mu == tm, im == mu, mu == im, tm == Map(1 -> "a")))
    println(List(tm.hashCode == im.hashCode, mu.hashCode == im.hashCode))
    calls = 0
    val copied = TreeMap.from(tm)
    println(List(calls, copied eq tm))
    val set = TreeSet(1, 2, 3)
    calls = 0
    val cs = TreeSet.from(set)
    println(List(calls, cs eq set))
    val other = TreeMap.from(tm)(using Ordering.Int.reverse)
    println(List(other.toList, other eq tm))
