import scala.language.implicitConversions
trait Ord[A]
object Far:
  implicit def fallback(a: Int): Fallback = new Fallback(a)
class Fallback(a: Int):
  def m(b: Int): Int = a * 10 + b
  def k(b: Int): Int = a * 100 + b
object Near:
  implicit def toOps[A](a: A)(using Ord[A]): Ops[A] = new Ops(a)

object Main:
  import Far.*
  import Near.*

  def main(args: Array[String]): Unit =
    println(3.m {
      class L[B](x: B):
        def k(y: B) = List(x, y).head
      implicit def toL[B](x: B)(using Ord[B]): L[B] = new L(x)
      5.k(6)
    })
    println(new Ops("s").m("t"))
class Ops[A](a: A):
  def m(b: A) = List(a, b).head
