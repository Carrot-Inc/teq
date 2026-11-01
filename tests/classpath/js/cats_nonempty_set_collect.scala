// jars: scala-library cats-kernel-sjs cats-core-sjs
// cats' `NonEmptySetOps.collect`, whose pickled body passes its local implicit to the sorted set's `collect`
// (`toSortedSet.collect[B](pf)(ordering)`, scala-library's `SortedSetOps.collect` beside `IterableOps.collect`), and
// the `Reducible` instance `NonEmptySetInstances`, a jar class, initialises in its constructor.
//> using dep org.typelevel::cats-core:2.13.0
import cats.Reducible
import cats.data.NonEmptySet
import cats.implicits.*
object Main:
  def main(args: Array[String]): Unit =
    val s = NonEmptySet.of(3, 1, 2)
    println(s.collect { case x if x > 1 => x * 10 })
    println(s.map(_ + 1))
    println(Reducible[NonEmptySet].reduceLeftTo(s)(_.toString)(_ + _))
