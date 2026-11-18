import scala.language.implicitConversions
trait Ord[A]
object Far:
  implicit def fallback(a: Int): Fallback = new Fallback(a)
class Fallback(a: Int):
  def m(b: Int): Int = a * 10 + b
  def e: Int = 1
  def n: Int = 2
  def f: Int = 3
  def p: Int = 4
object Near:
  implicit def toOps[A](a: A)(using Ord[A]): Ops[A] = new Ops(a)
object Main:
  import Far.*
  def main(args: Array[String]): Unit =
    import Near.*
    println(3.e)
    println(3.n)
    println(3.f)
    println(3.p)
class Fallback2
class Ops[A](a: A):
  def m(b: A) = a
  def e = List()
  def n = None
  def f = Map()
  def p = (a, List())
