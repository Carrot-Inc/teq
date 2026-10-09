// A using parameter summoned inside the body is the given of its declared type (`summon[B]`),
// but the value keeps the argument's own type: the deferred `apply` reaches `C`.
trait B:
  inline def apply(): String
class C extends B:
  inline def apply(): String = "ok"
class D extends B:
  inline def apply(): String = "d"
inline def inner(x: B): String = x()
inline def outer(using x: B): String = inner(summon[B])
inline def twice(using x: B): String = inner(summon[B]) + inner(x)
@main def run(): Unit =
  println(outer(using new C))
  println(twice(using new D))
  given C = new C
  println(outer)
