// expect: 7:68: error: type mismatch: found Any, required String
// expect: 1 error found
// A refinement narrows a call of the member it describes only: `c.f()` falls back to the
// extension, whose result stays `Any` (scalac: `Found: (r : Any), Required: String`).
trait C { def f(x: Int): Any }
extension (c: C) def f(): Any = 42
def bad(c: C { def f(x: Int): String }): String = { val r = c.f(); r }
class D extends C { def f(x: Int): String = "s" }
@main def run(): Unit = println(bad(new D))
