// jars: quote-patterns33-lib
// A Scala 3.3.7 library's quote pattern binding a type in a contravariant position, which scalac marks `@fromAbove`:
// the variable is the bound the match gives it from above, the argument's type.
import quotepatterns33.*

@main def run(): Unit =
  println(Shapes.shapeOf[Nothing](Pipe[Int, Nothing, Boolean]("a")))
  println(Shapes.shapeOf[Nothing](Pipe[Long, Nothing, Char]("b")))
