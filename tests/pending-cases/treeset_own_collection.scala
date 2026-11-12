// scala-library's `immutable.TreeSet` is `SortedSetOps[A, TreeSet, TreeSet[A]]`:
// its `filter`, `rangeFrom`, `drop` and a for comprehension's
// `withFilter` give a `TreeSet`. Those of teq's std `TreeSet`
// are to give one too. Waiting on the std's sorted sets
// taking scala-library's `SortedSetOps` type parameters.
import scala.collection.immutable.TreeSet

@main def run(): Unit =
  val t = TreeSet(3, 1, 2)
  val f: TreeSet[Int] = t.filter(_ > 1)
  val r: TreeSet[Int] = t.rangeFrom(2)
  val d: TreeSet[Int] = t.drop(1)
  println(s"$f $r $d")
  val w: TreeSet[Int] = for x <- t if x > 1 yield x * 10
  println(w)
