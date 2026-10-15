package mpb

import scala.deriving.Mirror
import mpa.*

@main def run(): Unit =
  println(summon[Mirror.ProductOf[D]].fromProduct(Tuple1(5)))
  println(Lib.mirror.fromProduct(Tuple1(6)))
  println(D.fromProduct(Tuple1(7)))
  val m: Mirror.Product = D
  println(m.fromProduct(Tuple1(8)))
