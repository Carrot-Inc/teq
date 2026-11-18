import scala.language.implicitConversions
trait Ord[A]
object Far:
  implicit def fallback(a: Int): Fallback = new Fallback(a)
class Fallback(a: Int):
  def foo(b: Int): Int = a * 10 + b
object Near:
  implicit def toOps[A](a: A)(using Ord[A]): Ops[A] = new Ops(a)
object Main:
  def identity[X](x: X): X = x
  import Far.*
  def main(args: Array[String]): Unit =
    import Near.*
    println(identity(3.foo(4)))
class Ops[A](a: A):
  def foo(b: A) = List(a, b).head
