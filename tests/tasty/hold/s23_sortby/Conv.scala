import scala.language.implicitConversions
trait Ord[A]
object Far:
  implicit def fallback(a: Int): Fallback = new Fallback(a)
class Fallback(a: Int):
  def m(b: Int): Int = a * 10 + b
object Near:
  implicit def toOps[A](a: A)(using Ord[A]): Ops[A] = new Ops(a)
