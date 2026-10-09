// teq: --inline-substitution
// An inline method defined inside a body resolves at its definition as a top-level one does
// (tests/cases/inline_definition_resolution): `pick(wrap(x).head)` keeps `pick(x: B)`, so `g(O)`
// prints `base` under scalac 3.8.4, as the expansion by substitution of the checked local method
// prints; the retype path types the local body again at the argument's own type
// and prints `object`.
trait B
object O extends B
def pick(x: B): String = "base"
def pick(x: O.type): String = "object"
@main def run(): Unit =
  inline def wrap(x: B): List[x.type] = List(x)
  inline def g(x: B): String = pick(wrap(x).head)
  inline def h(x: B): String = pick(x)
  println(g(O))
  println(h(O))
