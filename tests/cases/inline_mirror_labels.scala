// Labels and element counts from a mirror's tuples through recursive inline matches on
// tuple patterns, with an accumulating argument.
package inlinemirrorlabels

import scala.deriving.Mirror
import scala.compiletime.*

object Lab:
  inline def labels[Labels <: Tuple, Params <: Tuple](idx: Int): List[String] = inline erasedValue[(Labels, Params)] match
    case _: (EmptyTuple, EmptyTuple) => Nil
    case _: ((l *: ltail), (p *: ptail)) => (constValue[l].asInstanceOf[String] + idx) :: labels[ltail, ptail](idx + 1)
  inline def fromMirror[A](product: Mirror.ProductOf[A]): List[String] = labels[product.MirroredElemLabels, product.MirroredElemTypes](0)
  inline def count[A, Subs <: Tuple](m: Mirror.SumOf[A]): Int = inline erasedValue[Subs] match
    case _: EmptyTuple => 0
    case _: (s *: tail) => 1 + count[A, tail](m)
  inline def derived[A](using m: Mirror.Of[A]): String = inline m match
    case p: Mirror.ProductOf[A] => fromMirror[A](p).mkString(",")
    case s: Mirror.SumOf[A] => count[A, s.MirroredElemTypes](s).toString

case class Address(street: String, city: String, zip: Option[String])
sealed trait Shape
case class Circle(r: Int) extends Shape
case object Dot extends Shape

@main def main(): Unit =
  println(Lab.derived[Address])
  println(Lab.derived[Shape])
