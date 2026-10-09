// A function literal applied where it is written is reduced to its body with the argument for
// its parameter, a copy of the body: a plain inline call in it is expanded first, so that no copy
// keeps a call of an inline method.
object M:
  inline def one: Int = 1

@main def main(): Unit =
  println(((x: Int) => x + M.one)(41))
