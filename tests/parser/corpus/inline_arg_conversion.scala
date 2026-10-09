// A conversion of a parameter of an inline body is chosen at the declared type, as scalac
// chose it at the definition: `Conversion[B, BaseOps]`, not the argument's `Conversion[O.type, ObjectOps]`.
import scala.language.implicitConversions
trait B
object O extends B
class BaseOps:
  def tag(n: Int): String = "base" + n
class ObjectOps:
  def tag(n: Int): String = "object" + n
given Conversion[B, BaseOps] with
  def apply(x: B): BaseOps = new BaseOps
given Conversion[O.type, ObjectOps] with
  def apply(x: O.type): ObjectOps = new ObjectOps
inline def f(x: B): String = x.tag(1)
inline def g(inline x: B): String = x.tag(2)
@main def run(): Unit =
  println(f(O))
  println(g(O))
