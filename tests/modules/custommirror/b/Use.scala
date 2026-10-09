package cmb

@main def run(): Unit =
  val m: scala.deriving.Mirror.Product = cma.Custom
  println(m.fromProduct(EmptyTuple))
