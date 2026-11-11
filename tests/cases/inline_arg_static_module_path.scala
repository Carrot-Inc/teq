// An inline method's argument that is a stable path through a static object (`O.v`, `P.c.w`, `Q.R.r`) is bound by
// its singleton type, with no proxy val (Inliner.computeParamBindings), and each read of the parameter in the body is
// that path again (`tpd.singleton`). The path's object is `ref` of its module, the module itself, also where the
// inline method is a member of that object and its body stands in the object's scope: never the object's `this`,
// which the call's site has none of.
object O:
  val v = 1
  inline def h(x: Int) = x
  inline def twice(x: Int) = h(x) + h(x)
object A:
  object B:
    val v = 2
class C:
  val w = 3
object P:
  val c = new C
  inline def k(x: Int) = x
object Q:
  object R:
    val r = 4
    inline def i(x: Int) = x
import O.*
@main def run(): Unit =
  println(O.h(O.v))
  println(h(v))
  println(O.twice(O.v))
  println(O.h(A.B.v))
  println(P.k(P.c.w))
  println(Q.R.i(Q.R.r))
