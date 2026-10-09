// `Duration(text)` parses a length and a unit as scala-library does, and `Duration(n, "unit")` names the unit.
import scala.concurrent.duration.*

@main def main(): Unit =
  println(Duration("5 seconds"))
  println(Duration("1.5h"))
  println(Duration("100ms"))
  println(Duration("2 days"))
  println(Duration("Inf"))
  println(Duration("-Inf"))
  println(Duration("3 micros"))
  println(Duration(7, "min"))
  println(Duration("5 seconds").isFinite)
  println(scala.util.Try(Duration("5 lightyears")).isFailure)
