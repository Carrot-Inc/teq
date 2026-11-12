// Two top-level givens named alike (`given_Show_Box`,
// as scalac names both: only a type argument's head is
// used) are a double definition. dotty's `<file>$package`
// scope keeps both symbols and reports E161 once (Namer's
// `checkNoConflict`), and scalac 3.8.4 prints that one
// error. Queued with the port of the `$package` membership.
package demo

trait Show[A]:
  def show(a: A): String

case class Box[A](a: A)

// scalac calls both given_Show_Box: only a type argument's head is used.
given Show[Box[Int]] with
  def show(a: Box[Int]): String = "int"

given Show[Box[String]] with
  def show(a: Box[String]): String = "string"

@main def run(): Unit =
  println(summon[Show[Box[Int]]].show(Box(1)))
