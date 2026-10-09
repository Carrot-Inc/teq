// The scalac side of jar_erasure_signature.scala: compiled into a jar by scalac 3.8.4, never by
// teq. Types whose erasure is decided by what the class says, and the descriptors of the
// program's methods as reflection finds them.
package erasure

opaque type Id = Long
object Id:
  def apply(l: Long): Id = l

opaque type Tags[A] = List[A]
object Tags:
  def of[A](xs: A*): Tags[A] = xs.toList

final class Cm(val v: Int) extends AnyVal

object Descriptors:
  def of(className: String, method: String): String =
    val m = Class.forName(className).getMethods.find(_.getName == method).get
    m.getParameterTypes.map(_.getName).mkString(method + "(", ", ", "): ") + m.getReturnType.getName

// Overloads told apart by a type parameter's bound, which their descriptors are: `f[String]` is
// the first, `(String)I`, not the second, `(Object)I`.
object Bounded:
  def f[T <: String](x: T): Int = 1
  def f(x: Object): Int = 2
  def g[T <: Int](x: T): String = "int " + x
  def g(x: Object): String = "object " + x
  def arr[T <: String](xs: Array[T]): Int = xs.length
  def arr(xs: Object): Int = -1
  def wrap[T <: Cm](x: T): Int = x.v
  def wrap(x: Object): Int = -2

trait Named:
  def name: String
class BoundBox[T <: Named](t: T):
  def get: T = t
  def put(x: T): String = "box " + x.name
