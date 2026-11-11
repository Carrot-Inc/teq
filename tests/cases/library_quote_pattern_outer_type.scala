// jars: quotenest-lib
// A library macro's nested quote pattern that names the type its enclosing pattern bound: the body's pickled pattern
// refers to that type, and the splice of the outer pattern's expression keeps it.
import qnest.*

@main def run(): Unit =
  val a = Pipe[Int, Nothing, String]("a")
  val b = Pipe[String, Nothing, Boolean]("b")
  println(Mac.joined[Nothing](a, b))
