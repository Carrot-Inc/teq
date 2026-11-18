import scala.language.implicitConversions
trait Ord[A]
object Far:
  implicit def fallback(a: Int): Fallback = new Fallback(a)
class Fallback(a: Int):
  def foo(b: Int): Int = a * 10 + b
object Near:
  implicit def toOps[A](a: A)(using Ord[A]): Ops[A] = new Ops(a)
object Main:
  import Far.*
  def main(args: Array[String]): Unit =
    import Near.*
    println(3.foo(4))
class Ops[A](a: A):
  def foo(b: A) = identity(List(a, b)).head

def identity[T](x: T): T = x
