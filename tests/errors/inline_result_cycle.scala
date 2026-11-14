// Two inline methods whose results are inferred and that call each other: each result needs the other's, a cycle
// scalac reports as E044 (`Overloaded or recursive method outer needs return type`) where the second reads the
// first; a method that calls itself is the same cycle.
// expect: error: recursive use of outer needs an explicit result type
// expect: error: recursive use of alone needs an explicit result type
object Lib:
  inline def outer(x: Int) = inner(x) + 1
  inline def inner(x: Int) = if x > 0 then outer(x - 1) else 0
  inline def alone(x: Int) = if x > 0 then alone(x - 1) else 0
@main def run(): Unit = println(Lib.outer(0) + Lib.alone(0))
