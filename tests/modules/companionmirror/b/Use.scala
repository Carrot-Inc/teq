package cbb

@main def run(): Unit =
  val m: scala.deriving.Mirror.Product = cba.Box
  println(m.fromProduct(Tuple1(1)))
