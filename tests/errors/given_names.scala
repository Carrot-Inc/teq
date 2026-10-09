// expect: given_Show_Box is already defined
package demo

trait Show[A]:
  def show(a: A): String

case class Box[A](a: A)

// Both are called given_Show_Box, as in scalac: only the head of a type argument is used.
given Show[Box[Int]] with
  def show(a: Box[Int]): String = "int"

given Show[Box[String]] with
  def show(a: Box[String]): String = "string"

@main def run(): Unit =
  println(summon[Show[Box[Int]]].show(Box(1)))
