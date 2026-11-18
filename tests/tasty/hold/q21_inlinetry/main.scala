import scala.language.implicitConversions
trait Ord[A]
object Far:
  implicit def fallback(a: Int): Fallback = new Fallback(a)
class Fallback(a: Int):
  def m(b: Int): Int = a * 10 + b
object Near:
  implicit def toOps[A](a: A)(using Ord[A]): Ops[A] = new Ops(a)

object Main:
  import Far.*
  import Near.*
  inline def go(x: Int): Int =
    x.m(4)
  def main(args: Array[String]): Unit =
    println(go(3))
    println(new Ops("s").m("t"))
class Ops[A](a: A):
  def m(b: A) = List(a, b).head
