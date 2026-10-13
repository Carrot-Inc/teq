// A class's given with type parameters and no term clause is a def (dotty's `Parsers.givenDef`):
// its right-hand side runs at each call, whatever the type argument.
class C:
  var n = 0
  given g[A]: Int = { n += 1; n }
@main def run(): Unit =
  val c = new C
  println(c.g[String])
  println(c.g[Int])
  println(c.g[Int])
