// Two top-level givens named alike (`given_Show_Box`, as scalac names both: only the head of a type argument is used)
// are a double definition. dotty's `<file>$package` scope keeps both symbols and reports E161 once (Namer's
// `checkNoConflict`), and scalac 3.8.4 prints that one error. master (efddb38de3) prints one error too; the integration
// base (cf6fd6d08) and the implicits helper's tip (93eeea276d) print two, adding "type given_Show_Box$ is already
// defined" for the given object's module class: a TR regression. The package's entries keep one term per name. Queued
// with the port of the `$package` membership.
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
