// scala-library's `SortedSetOps.collect` and `zip` take an `Ordering` for their result beside `IterableOps`'s
// `collect` and `zip`: a call with the evidence found, or passed explicitly as cats' pickled `NonEmptySetOps.collect`
// body passes it (`toSortedSet.collect[B](pf)(ordering)`), is the sorted set's, on the immutable and the mutable
// sets, and a `TreeSet`'s gives a `TreeSet` (its `CC`), as its `map` and `flatMap` do; through the static type `Set`
// no ordering is passed and the result is a plain set.
import scala.collection.immutable.{SortedSet, TreeSet}
import scala.collection.mutable

object Main:
  @annotation.nowarn("msg=Implicit parameters should be provided with a `using` clause")
  def collected[B](s: SortedSet[Int], pf: PartialFunction[Int, B])(using B: Ordering[B]): SortedSet[B] =
    implicit val ordering: Ordering[B] = B
    s.collect[B](pf)(ordering)
  def main(args: Array[String]): Unit =
    val pf: PartialFunction[Int, Int] = { case x if x > 1 => x * 10 }
    val s = SortedSet(3, 1, 2)
    println(collected(s, pf))
    val c: SortedSet[Int] = s.collect(pf)
    println(c)
    println(s.collect(pf)(using Ordering[Int].reverse))
    val z: SortedSet[(Int, String)] = s.zip(List("c", "a", "b"))
    println(z)
    val asSet: Set[Int] = s
    println(s"${asSet.collect(pf).isInstanceOf[SortedSet[?]]} ${asSet.collect(pf).toList.sorted}")
    val m = mutable.TreeSet(3, 1, 2)
    println(m.collect(pf))
    println(m.zip(List(9, 8, 7)))
    val t = TreeSet(3, 1, 2)
    val tc: TreeSet[Int] = t.collect(pf)
    val tz: TreeSet[(Int, Int)] = t.zip(List(9, 8, 7))
    val tm: TreeSet[Int] = t.map(_ + 1)
    val tf: TreeSet[Int] = t.flatMap(x => List(x, -x))
    println(s"$tc $tz $tm $tf")
    val mc: mutable.TreeSet[Int] = m.collect(pf)
    val mz: mutable.TreeSet[(Int, Int)] = m.zip(List(9, 8, 7))
    val mm: mutable.TreeSet[Int] = m.map(_ + 1)
    val mf: mutable.TreeSet[Int] = m.flatMap(x => List(x, -x))
    println(s"$mc $mz $mm $mf")
