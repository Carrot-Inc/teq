// expect: 8:75: error: type mismatch: found Any, required String
// A refinement of a receiver says nothing of a call on what a conversion makes of it. The member's
// application does not type (`""` is no `Int`), so the receiver is converted, as scalac's
// `tryInsertImplicitOnQualifier` has it, and `v` is rejected (`Found: (v : Any), Required: String`).
import scala.language.implicitConversions
class C[A] { def f(x: A): Any = 0 }
implicit def cv(c: C[Int]): C[String] = new C[String]
def bad(c: C[Int] { def f(x: Int): String }): String = { val v = c.f(""); v }
@main def Main(): Unit = println("ok")
