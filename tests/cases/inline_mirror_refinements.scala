// jars: scala-library
import scala.deriving.Mirror
import scala.compiletime.{constValue, constValueTuple, summonAll, summonFrom, erasedValue}

inline def mirrorLabel[A](using m: Mirror.Of[A]): String =
  inline m match
    case p: Mirror.ProductOf[A] => constValue[p.MirroredLabel]
    case s: Mirror.SumOf[A] => constValue[s.MirroredLabel]

inline def mirrorLabels[A](using m: Mirror.Of[A]): Tuple =
  inline m match
    case p: Mirror.ProductOf[A] => constValueTuple[p.MirroredElemLabels]
    case s: Mirror.SumOf[A] => constValueTuple[s.MirroredElemLabels]

inline def mirrorSize[A](using m: Mirror.Of[A]): Int =
  inline m match
    case p: Mirror.ProductOf[A] => constValue[Tuple.Size[p.MirroredElemTypes]]
    case s: Mirror.SumOf[A] => constValue[Tuple.Size[s.MirroredElemTypes]]

trait Element[A]
given Element[Int] = new Element[Int] {}
given Element[String] = new Element[String] {}
inline def elements[A](using m: Mirror.ProductOf[A]): Tuple =
  summonAll[Tuple.Map[m.MirroredElemTypes, Element]]

inline def resolveElement[A]: String = summonFrom {
  case element: Element[A] => "given"
  case mirror: Mirror.Of[A] => "mirror"
}
inline def collectElements[Ts <: Tuple]: List[String] = inline erasedValue[Ts] match
  case _: EmptyTuple => Nil
  case _: (head *: tail) => resolveElement[head] :: collectElements[tail]

type LargeTuple = (Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int)

inline def mirrorLabelList[A](using m: Mirror.Of[A]): List[String] =
  inline m match
    case p: Mirror.ProductOf[A] => constValueTuple[p.MirroredElemLabels].toList.asInstanceOf[List[String]]
    case s: Mirror.SumOf[A] => constValueTuple[s.MirroredElemLabels].toList.asInstanceOf[List[String]]
inline def nestedLabels[A]: List[String] = summonFrom {
  case m: Mirror.Of[A] => mirrorLabelList[A](using m)
}
inline def collectLabels[Ts <: Tuple]: List[List[String]] = inline erasedValue[Ts] match
  case _: EmptyTuple => Nil
  case _: (head *: tail) => nestedLabels[head] :: collectLabels[tail]

case class EmptyPoint()
case class Point(x: Int, y: String)
enum Choice:
  case First, Last

@main def run(): Unit =
  println(mirrorLabel[Point])
  println(mirrorLabel[Choice])
  println(mirrorLabels[Point])
  println(mirrorLabels[Choice])
  println(mirrorSize[Point])
  println(mirrorSize[Choice])
  println(elements[Point].productArity)
  println(collectElements[(Int, Point, String)])
  println(constValue[Tuple.Size[LargeTuple]])
  println(collectLabels[(Point, EmptyPoint)])
