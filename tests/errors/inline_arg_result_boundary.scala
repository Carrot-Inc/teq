// expect: 12:33: error: Deferred inline method apply in trait B cannot be invoked
// expect: 1 error found
// An inline call's result carries no parameter of the body around it: `id(id(x))` is a `B`,
// on which the extension's inline binding cannot reach `C`, as scalac refuses it.
trait B:
  inline def apply(): String
class C extends B:
  inline def apply(): String = "ok"
extension [A <: B](x: A) inline def tag: String = x()
inline def id[A](x: A): A = x
inline def f(x: B): String = id(id(x)).tag
@main def run(): Unit = println(f(new C))
