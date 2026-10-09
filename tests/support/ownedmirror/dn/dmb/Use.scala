package dmb

import scala.deriving.Mirror
import scala.compiletime.constValue
import dma.*

inline def label[A](using m: Mirror.Of[A]): String = constValue[m.MirroredLabel]

// The mirrors of the upstream's classes, which only this project summons: the upstream's build
// writes them all the same.
@main def run(): Unit =
  println(summon[Mirror.ProductOf[Point]].fromProduct((1, 2)))
  println(summon[Mirror.ProductOf[Box[Int]]].fromProduct(Tuple1(3)))
  println(summon[Mirror.SumOf[Shape]].ordinal(Rect(1.0, 2.0)))
  println(summon[Mirror.SumOf[Color]].ordinal(Color.Green))
  println(summon[Mirror.ProductOf[Color.Rgb]].fromProduct((1, 2, 3)))
  println(summon[Mirror.ProductOf[Color.Red.type]].fromProduct(EmptyTuple))
  println(summon[Mirror.ProductOf[Origin.type]].fromProduct(EmptyTuple))
  println(summon[Mirror.ProductOf[Shapes.Square]].fromProduct(Tuple1(2.0)))
  println(label[Circle] + " " + label[Color] + " " + label[Origin.type])
