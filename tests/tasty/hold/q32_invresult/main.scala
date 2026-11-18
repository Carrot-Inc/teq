import scala.language.implicitConversions
trait Ord[A]
object Far:
  implicit def fallback(a: Int): Fallback = new Fallback(a)
class Fallback(a: Int):
  def m(b: Int): Int = a * 10 + b
object Near:
  implicit def toOps[A](a: A)(using Ord[A]): Ops[A] = new Ops(a)
object Mk:
  def set[T]: Set[T] = Set.empty
  def arr[T](xs: T*)(using scala.reflect.ClassTag[T]): Array[T] = xs.toArray
object Main:
  import Far.*
  import Near.*

  def main(args: Array[String]): Unit =
    println(3.m(4))
    println(new Ops("s").m("t"))
class Ops[A](a: A):
  def m(b: A) = Mk.set
