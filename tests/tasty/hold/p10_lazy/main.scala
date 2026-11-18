import scala.language.implicitConversions
trait Ord[A]
object Far:
  implicit def fallback(a: Int): Fallback = new Fallback(a)
class Fallback(a: Int):
  def m: Int = a * 10
object Near:
  implicit def toOps[A](a: A)(using Ord[A]): Ops[A] = new Ops(a)

object Main:
  import Far.*

  def main(args: Array[String]): Unit =
    import Near.*
    println(3.m)
    println(new Ops("s").m)
class Ops[A](a: A):
  lazy val m = List(a).head
