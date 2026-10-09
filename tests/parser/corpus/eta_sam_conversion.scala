// A method eta-expanded into a trait with a single abstract method takes the trait method's
// parameter, which the argument adaptations take to the method's: an implicit conversion as
// well as a numeric widening (dotc's `(x: A) => m(convert(x))`).
import scala.language.implicitConversions
class A(val n: Int)
class B(val n: Int)
given Conversion[A, B] with
  def apply(a: A): B = B(a.n * 10)
trait S:
  def run(x: A): Int
trait W:
  def run(x: Int): Long
trait R:
  def run(x: A): B
def same(x: A): A = x
def m(x: B): Int = x.n + 1
def w(x: Long): Long = x * 2
@main def run(): Unit =
  val result: S = m
  println(result.run(A(4)))
  val widened: W = w
  println(widened.run(21))
  val converted: R = same
  println(converted.run(A(5)).n)
