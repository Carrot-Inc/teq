// A class that derives CanEqual compares where its type arguments compare pairwise, as the instance scalac
// derives has it (`CanEqual[Box[L], Box[R]]` given a `CanEqual[L, R]`), under strict equality too: a pattern
// and `==` over a `Box` of a bounded member against a `Box[String]`.
import scala.language.strictEquality
case class Box[A](value: A) derives CanEqual
trait Test:
  type A <: String
  def pattern(x: Box[A], y: Box[String]): Boolean = x match { case `y` => true; case _ => false }
  def equal(x: Box[A], y: Box[String]): Boolean = x == y
  def nested(x: Box[Box[A]], y: Box[Box[String]]): Boolean = x != y
object Main:
  def main(args: Array[String]): Unit =
    val t = new Test { type A = String }
    println(List(t.pattern(Box("a"), Box("a")), t.pattern(Box("a"), Box("b")), t.equal(Box("a"), Box("a")),
      t.nested(Box(Box("a")), Box(Box("b"))), Box(1) == Box(2), Box(Box(1)) == Box(Box(1))).mkString(" "))
