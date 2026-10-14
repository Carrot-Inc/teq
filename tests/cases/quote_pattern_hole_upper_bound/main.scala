@main def main(): Unit =
  println(M.unwrap(M.id("yes")))
  println(M.unwrap(M.any("x")))
  println(M.inside)
  println(M.unwrap(1))
