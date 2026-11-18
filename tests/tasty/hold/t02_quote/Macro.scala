import scala.language.implicitConversions
import scala.quoted.*

trait Ord[A]
object Far:
  implicit def fallback(a: Int): Fallback = new Fallback(a)
class Fallback(a: Int):
  def m(b: Int): Int = a * 10 + b
object Near:
  implicit def toOps[A](a: A)(using Ord[A]): Ops[A] = new Ops(a)
class Ops[A](a: A):
  def m(b: A) = List(a, b).head

object Q:
  inline def run: Int = ${ runImpl }
  def runImpl(using Quotes): Expr[Int] =
    import Far.*
    import Near.*
    '{ 3.m(4) }
