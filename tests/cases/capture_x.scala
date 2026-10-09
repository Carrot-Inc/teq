// A conversion method that fills an implicit function value is expanded to a lambda whose
// parameter is named `x$0`, as the method is.
import scala.language.implicitConversions

given String = "#"
implicit def `x$0`(i: Int)(using prefix: String): String = prefix + i
def show(i: Int)(implicit f: Int => String): String = f(i)

@main def main(): Unit =
  println(show(3))
