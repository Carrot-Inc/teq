@main def main(): Unit =
  println(M.shape[Any, Int] { case n: Int => n; case s: String => s.length })
