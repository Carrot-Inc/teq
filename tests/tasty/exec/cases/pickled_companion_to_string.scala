// An inline body selecting a value class companion's `toString()`, which scala-library declares with an empty
// parameter list (`Int.scala`): the selection is pickled with that member's shape and the body is written.
object Lib:
  inline def name: String = scala.Int.toString
  inline def names: String = scala.Boolean.toString + " " + scala.Long.toString + " " + scala.Double.toString

@main def run(): Unit =
  println(Lib.name)
  println(Lib.names)
