// expect: 8:43: error: index out of bounds: 2
// expect: 1 error found
// A plain inline call of a constant type inside a tuple's index makes it static: past the elements
// it is a compile-time error, as scalac reports it.
object M:
  inline def two: 2 = 2

@main def run(): Unit = println((42, "s")(M.two + 0))
