package dmb2

import scala.deriving.Mirror
import dma2.*

@main def run(): Unit =
  println(summon[Schema[Email]].make(Tuple1("ok")))
  println(summon[Schema[Point]].make((3, 4)))
  println(Library.mirror.fromProduct((5, 6)))
  println(summon[Mirror.ProductOf[Email]].fromProduct(Tuple1("direct")))
  println(Mac.made)
