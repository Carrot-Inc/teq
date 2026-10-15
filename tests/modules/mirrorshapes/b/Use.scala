package msb

import scala.deriving.Mirror
import msa.*

given Make[String] = new Make[String]:
  def make(p: Product): String = p.productElement(0).toString

@main def run(): Unit =
  println(Use.plain.fromProduct((1, "a")))
  println(Use.generic.fromProduct(Tuple1(7)))
  println(s"${Use.sum.ordinal(Sum.One)} ${Use.sum.ordinal(Sum.Many(2))}")
  println(Use.one.fromProduct(EmptyTuple))
  println(Use.many.fromProduct(Tuple1(3)))
  println(s"${Use.shape.ordinal(Circle(1))} ${Use.shape.ordinal(Dot)}")
  println(Use.tagged.ordinal(Blue()))
  println(Use.lone.fromProduct(EmptyTuple))
  println(Use.pair.fromProduct((4, "p")))
  println(summon[Make[Plain]].make((2, "b")))
  println(summon[Make[Generic[String]]].make(Tuple1("g")))
  println(summon[Index[Sum]].index(Sum.Many(1)))
  println(summon[Mirror.ProductOf[Plain]].fromProduct((3, "c")))
  println(summon[Mirror.SumOf[Shape]].ordinal(Dot))
  println(summon[Mirror.SumOf[Tagged]].ordinal(Red(1)))
  println(summon[Mirror.ProductOf[Lone.type]].fromProduct(EmptyTuple))
  println(summon[Mirror.ProductOf[Generic[String]]].fromProduct(Tuple1("h")))
