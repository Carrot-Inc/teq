// Double.equals compares the bits where == compares the values.
@main def main(): Unit =
  val nan = 0.0 / 0.0
  println(nan == nan)
  println(nan.equals(nan))
  println(0.0 == -0.0)
  println((0.0).equals(-0.0))
  println((1.5).equals(1.5))
  println((1.5).equals(2.5))
  println(List(nan).contains(nan))
  println(List(nan).indexOf(nan))
  println(List(0.0, -0.0).distinct.size)
  println(Set(0.0, -0.0).size)
  println(Math.abs(Int.MinValue))
  println(Math.abs(Long.MinValue))
  println(math.abs(-3) + " " + Math.abs(-3L) + " " + math.abs(-2.5) + " " + Int.MinValue.abs + " " + (-7).abs + " " + Math.abs(7))
