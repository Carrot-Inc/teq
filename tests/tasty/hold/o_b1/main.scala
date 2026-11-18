import scala.language.implicitConversions
trait Ord[A]
object Far:
  implicit def fallback(a: Int): Fallback = new Fallback(a)
class Fallback(a: Int):
  def min(b: Int): Int = a
object Near:
  implicit def toOps[A](a: A)(using Ord[A]): Ops[A] = new Ops(a)
object Main:
  import Far.*
  def main(args: Array[String]): Unit =
    import Near.*
    println(3.min(4))
class Ops[A](a: A):
  def min(b: A) = List(a, b).head
