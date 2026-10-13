package mptb

import scala.deriving.Mirror
import mpta.*

case class S(n: Int, s: String)
case class T(x: Int, y: String)

@main def run(): Unit =
  println(Convert.toQ(R(1, "one")))
  println(summon[Mirror.ProductOf[S]].fromProductTyped(T(2, "two")))
  println(summon[Mirror.ProductOf[T]].fromProductTyped((3, "three")))
