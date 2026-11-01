// scala-library's `immutable.TreeSet` is `SortedSetOps[A, TreeSet, TreeSet[A]]`: its `filter`, `rangeFrom`, `drop` and
// a for comprehension's `withFilter` give a `TreeSet`. teq's std `TreeSet` extends its `SortedSet[A]`, an
// `IterableOps[A, Set, SortedSet[A]]`, so those results are `SortedSet`s (`found SortedSet[Int], required
// TreeSet[Int]`; master the same), its `map`, `flatMap`, `collect` and `zip` refined one by one as its `incl` and std's
// `TreeMap`'s are. Waiting on the std's sorted sets taking scala-library's `SortedSetOps` type parameters.
import scala.collection.immutable.TreeSet

@main def run(): Unit =
  val t = TreeSet(3, 1, 2)
  val f: TreeSet[Int] = t.filter(_ > 1)
  val r: TreeSet[Int] = t.rangeFrom(2)
  val d: TreeSet[Int] = t.drop(1)
  println(s"$f $r $d")
  val w: TreeSet[Int] = for x <- t if x > 1 yield x * 10
  println(w)
