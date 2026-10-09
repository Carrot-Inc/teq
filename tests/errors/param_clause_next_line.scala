// expect: expected an identifier, found literal

// Without a result type the next line is read as a parameter clause, which is what scalac does.
trait T:
  def abs(a: Int)
  (1, 2).toString

@main def run(): Unit =
  println("x")
