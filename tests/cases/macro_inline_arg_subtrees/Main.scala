@main def run(): Unit =
  val k = 7
  println(Mac.describe((1 + 2) + k))
  println(Mac.describe(1 + 2))
  println(Mac.isLit(1 + 2))
  println(Mac.isLit(math.max(1 + 2, k)))
  println(Mac.isLit((y: Int) => y + (1 + 2)))
  println(W.wrap(k))
