import scala.language.implicitConversions
trait Ord[A]
object Far:
  implicit def fallback(a: Int): Fallback = new Fallback(a)
class Fallback(a: Int):
  def m(b: Int): Int = a * 10 + b
object Near:
  implicit def toOps[A](a: A)(using Ord[A]): Ops[A] = new Ops(a)
object Mk:
  def sink[T]: T => Int = _ => 1
  def empty[T]: List[T] = Nil
object Main:
  import Far.*
  import Near.*

  def main(args: Array[String]): Unit =
    println(3.m(4))
    println(new Ops("s").m("t"))
class Ops[A](a: A):
  def m(b: A) = Mk.empty
