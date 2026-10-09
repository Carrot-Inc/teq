// After Scala 3's tests/pos/spec-*.scala: annotations on type parameters, and
// @uncheckedVariance in a signature.
import scala.{specialized => sp}
import scala.specialized
import scala.annotation.unchecked.uncheckedVariance

trait Semigroup[@sp(Int, Long) A]:
  def combine(x: A, y: A): A

object Semigroup:
  given Semigroup[Int] with
    def combine(x: Int, y: Int): Int = x + y
  given Semigroup[String] with
    def combine(x: String, y: String): String = x + y

def fold[@specialized(Int, Long) A](xs: List[A])(using s: Semigroup[A], zero: A): A =
  xs.foldLeft(zero)(s.combine)

class Cell[@specialized(Specializable.Primitives) T](val value: T):
  def get: T = value

// A covariant parameter in a method parameter, which the annotation exempts from the check.
sealed trait Stream[+A]:
  def prepend(a: A @uncheckedVariance): Stream[A] = Cons(a, this)
  def toList: List[A] = this match
    case Cons(h, t) => h :: t.toList
    case Empty => Nil
case class Cons[+A](head: A, tail: Stream[A]) extends Stream[A]
case object Empty extends Stream[Nothing]

class Sink[-A]:
  def accept(a: A): String = a.toString
  def last: Option[A @uncheckedVariance] = None

def check[@unchecked T](x: Any): Boolean = x match
  case _: List[T @unchecked] => true
  case _ => false

@main def run(): Unit =
  given Int = 0
  println(fold(List(1, 2, 3)))
  given String = ""
  println(fold(List("a", "b")))
  println(new Cell(7).get)
  val s: Stream[Int] = (Empty: Stream[Int]).prepend(2).prepend(1)
  println(s.toList)
  println(new Sink[Int].accept(3))
  println(new Sink[Int].last)
  println(check[Int](List(1)))
  println(check[Int](3))
