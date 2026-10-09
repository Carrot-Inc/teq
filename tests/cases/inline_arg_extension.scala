// An extension on a parameter of an inline body is the declared type's, as scalac chose it at
// the definition (`x: B` takes the `B` extension, not the argument's `O.type` one), for a
// by-value and for an `inline` parameter alike.
trait B
object O extends B
class C extends B
extension (x: B) def tag(n: Int): String = "base" + n
extension (x: O.type) def tag(n: Int): String = "object" + n
extension (x: C) def tag(n: Int): String = "class" + n
inline def f(x: B): String = x.tag(1)
inline def g(inline x: B): String = x.tag(2)
@main def run(): Unit =
  println(f(O))
  println(g(O))
  println(f(new C))
  println(g(new C))
  println((new C).tag(3))
