@main def main() =
  println(M.mapped(M.f(1, 2)))
  println(M.mapped(M.g(1)(2, 3)))
  println(M.mapped(M.f()))
  val xs = Seq(4, 5)
  println(M.mapped(M.f(xs*)))
