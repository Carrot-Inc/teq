// jars: ctor-lib scala-library
// targets: js interp jvm
// A jar class's defs whose bodies call the sorted set's `collect` through its implicit clause (the pickle's
// `s.collect[B](pf)(ordering)`, scala-library's `SortedSetOps.collect` beside `IterableOps.collect`) and its `map` and
// `zip` with the class's given val, beside vals its constructor initialises; one of them, in a jar superclass of an
// object, an instance whose method calls such a def, as cats' `NonEmptySetInstances` makes its `Reducible`.
import scala.collection.immutable.SortedSet

@main def run(): Unit =
  val sorted = ctorlib.Sorted(SortedSet(3, 1, 2))
  println(s"${sorted.size} ${sorted.collected { case x if x > 1 => x * 10 }}")
  println(sorted.collected { case x if x > 1 => -x }(using Ordering[Int].reverse))
  println(sorted.zipped)
  println(ctorlib.SortedInstances.shows.show(SortedSet(5, 1, 4)))
