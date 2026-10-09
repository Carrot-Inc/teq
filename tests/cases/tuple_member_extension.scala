// A member of `Tuple` whose application does not type gives way to the program's extension of the
// name that applies, as scalac's `tryInsertImplicitOnQualifier` has it: an application's
// `settings.map(_.map(_.toLowerCase))` over pairs takes its own `map`, not `Tuple.map` of a
// polymorphic function; the builtins (`zip`, `splitAt`) whose argument does not type alone too.
extension [A](p: (A, A)) def map[B](f: A => B): (B, B) = (f(p._1), f(p._2))
extension (p: (Int, Int))
  def zip(f: Int => Int): Int = f(p._1) + f(p._2)
  def take(s: String): String = s * p._1
  def splitAt(f: Int => Boolean): Boolean = f(p._1)
  def ++(s: String): String = s + p._1
@main def test(): Unit =
  val settings = List(("Ab", "Cd"), ("Ef", "Gh"))
  println(settings.map(_.map(_.toLowerCase)))
  println(("X", "Y").map(_.toLowerCase))
  println((1, 2).zip(_ * 10))
  println((3, 4).take("ab"))
  println((7, 8).splitAt(_ > 5))
  println((9, 10) ++ "n")
  println((1, 2).map(identity).zip((3, 4)))
  println((1, 2).take(1))
  println((1, 2).reverse)
