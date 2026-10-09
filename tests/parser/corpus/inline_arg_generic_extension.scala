// A generic inline extension on a parameter is selected at the declared type (`A := B`) and
// its binding takes the receiver's own type: the deferred `apply` reaches `C`, an `inline
// match` on the receiver sees `C`.
trait B:
  inline def apply(): String
class C extends B:
  inline def apply(): String = "ok"
class D extends B:
  inline def apply(): String = "d"
extension [A <: B](x: A)
  inline def tag(n: Int): String = x() + n
  inline def kind: String = inline x match
    case _: C => "is C"
    case _ => "other"
inline def f(x: B): String = x.tag(1) + " " + x.kind
@main def run(): Unit =
  println(f(new C))
  println(f(new D))
