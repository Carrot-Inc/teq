import scala.language.implicitConversions
trait Ord[A]
object Far:
  implicit def fallback(a: Int): Fallback = new Fallback(a)
class Fallback(a: Int):
  def m(b: Int): Int = a * 10 + b
object Near:
  implicit def toOps[A](a: A)(using Ord[A]): Ops[A] = new Ops(a)

object Mk:
  inline def one[T](x: T): T = x
  transparent inline def tone[T](x: T): T = x
  def f(x: Int): Int = x
  def f(x: String): Int = x.length
object Main:
  import Far.*
  import Near.*

  def main(args: Array[String]): Unit =
    println(3.m(Mk.tone(4)))
    println(new Ops("s").m("t"))
class Ops[A](a: A):
  def m(b: A) = List(a, b).head
