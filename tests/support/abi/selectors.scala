// Item 1 of the JVM ABI alignment: a case class has scalac's
// selectors `_1` .. `_N`, one per parameter of the first clause, of the parameter's erased type, which scalac's
// pattern match on the class calls; none where the parameter is named so or the class defines the name.
// abi: _1 _2 _3 _23
package selectors
case class C[T](t: T)
case class C3[T <: Int](t: T)
case class Pair[T <: Int](t: T, u: T)
case class Empty()
case class Named(_1: Int, b: String)
case class Restricted(private val x: Int)(val y: Int)
case class BodyDef(a: Int, b: Int) { def _2: Int = 7 }
case class Rep(a: Int, rest: String*)
case class ByVar(var v: Int)
abstract case class Abs(a: Int)
sealed trait S
case class Sub(a: Long) extends S
trait P { def _1: Any }
case class D[T <: Int](t: T) extends P { override def _1: T = t }
case class Big(a1: Int, a2: Int, a3: Int, a4: Int, a5: Int, a6: Int, a7: Int, a8: Int, a9: Int, a10: Int, a11: Int, a12: Int, a13: Int, a14: Int, a15: Int, a16: Int, a17: Int, a18: Int, a19: Int, a20: Int, a21: Int, a22: Int, a23: String)
object O:
  case class Inner(q: Double)
class Outer:
  case class Path(p: Int)
enum Opt:
  case Some(x: Int)
  case None
