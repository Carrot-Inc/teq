// A given's extension is selected as `f(qual)` types: the using clauses before its own parameters are applied
// (`tryExtension`), so one without an instance passes the given over for another of the level. Same-named
// extensions of an instance resolve on the arguments typed alone (`resolveOverloaded`), type arguments written.
trait Missing
trait Ops:
  extension (x: Int) def f(using Missing)(n: Int): String = "bad"
trait Good:
  extension (x: Int) def f(n: Int): String = "good"
object A { given Ops = new Ops {} }
object B { given Good = new Good {} }
trait Pairs:
  extension [T](x: Int) def pair(y: (Double, Double), z: Boolean): String = "double"
  extension [T](x: Int) def pair(y: (Int, Int), z: Int): String = "int"
object P extends Pairs
@main def run(): Unit =
  import B.given
  locally { import A.given; println(1.f(2)) }
  println(P.pair[Unit](1)((1, 2), 3))
  println(P.pair[Unit](1)((1.0, 2.0), true))
  println(P.pair(1)((1, 2), 3))
