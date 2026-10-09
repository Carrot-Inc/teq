package amb

@main def run(): Unit =
  val m: scala.deriving.Mirror.Product = ama.Box
  println(m.fromProduct(EmptyTuple))
