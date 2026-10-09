// jars: scala-library deferred-lib
// The deferred and abstract givens of a scalac-built jar's traits (tests/support/deferred_lib.scala):
// a program class implements a deferred one by the search where it is defined, an explicit
// implementation overrides one, a parameterized one is implemented explicitly, an old-style abstract
// one by a given of the class, and a class extending the jar's implementing class inherits scalac's
// implementation. teq read the bodyless given as concrete and final: `AbstractMethodError`.
import dfl.*

object Site:
  given String = "site"
  class Plain extends Named
class Sorted extends Ordered[Int]
class Reversed extends Ordered[Int]:
  override given ord: Ordering[Int] = Ordering.Int.reverse
class WithShown extends Shown:
  override given shown(using n: Int): String = "n=" + n
class Decl extends Declared:
  given decl: Int = 21
class Sub extends Implemented(using "jar")

@main def run(): Unit =
  println((new Site.Plain).show)
  println(Sorted().sorted(List(3, 1, 2)))
  println(Reversed().sorted(List(3, 1, 2)))
  println(WithShown().shown(using 4))
  println(s"${Decl().decl} ${Decl().twice}")
  println(s"${Sub().name} ${Sub().show}")
