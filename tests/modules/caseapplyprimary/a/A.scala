package cap

// Case classes with a secondary constructor, read by module b from this module's products: their name applied
// there is the companion's synthesized `apply`, the primary alone (tests/cases/case_apply_primary_alone.scala).
case class G[T](x: T):
  def this(a: T, b: T) = this(a)
case class A(x: Any):
  def this(a: Int, b: Int) = this(a + b)
case class V(x: Int):
  def this(a: Int, b: Int) = this(a + b)
case class S(x: Any):
  def this(s: String) = this(s.length)
object O:
  case class K(x: Any):
    def this(a: Int, b: Int) = this(a + b)
trait Outer:
  case class In(x: Any):
    def this(a: Int, b: Int) = this(a + b)
inline def made(): String = G(1, 2).toString + A(3, 4).toString + V(5).x
