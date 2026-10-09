import scala.deriving.Mirror
import scala.compiletime.{constValue, constValueTuple, erasedValue, summonFrom, summonInline}

trait Show[T]:
  def show(x: T): String

object Show:
  given Show[Int] with
    def show(x: Int): String = x.toString
  given Show[String] with
    def show(x: String): String = "\"" + x + "\""

  inline def showElems[Elems <: Tuple, Labels <: Tuple](n: Int)(x: Product): List[String] =
    inline erasedValue[Elems] match
      case _: (elem *: elems1) =>
        inline erasedValue[Labels] match
          case _: (label *: labels1) =>
            val formal = constValue[label]
            val actual = summonInline[Show[elem]].show(x.productElement(n).asInstanceOf[elem])
            s"$formal = $actual" :: showElems[elems1, labels1](n + 1)(x)
      case _: EmptyTuple => Nil

  inline def showCase[T](x: T): String =
    summonFrom {
      case s: Show[T] => s.show(x)
      case m: Mirror.ProductOf[T] =>
        val label = constValue[m.MirroredLabel]
        val elems = showElems[m.MirroredElemTypes, m.MirroredElemLabels](0)(x.asInstanceOf[Product])
        if elems.isEmpty then label else label + elems.mkString("(", ", ", ")")
    }

  inline def showCases[Alts <: Tuple](n: Int)(x: Any, ord: Int): String =
    inline erasedValue[Alts] match
      case _: (alt *: alts1) =>
        if ord == n then showCase[alt](x.asInstanceOf[alt])
        else showCases[alts1](n + 1)(x, ord)
      case _: EmptyTuple => throw new MatchError(x)

  inline def derived[T](using m: Mirror.Of[T]): Show[T] = new Show[T]:
    def show(x: T): String =
      inline m match
        case s: Mirror.SumOf[T] => showCases[s.MirroredElemTypes](0)(x, s.ordinal(x))
        case p: Mirror.ProductOf[T] =>
          val label = constValue[p.MirroredLabel]
          val elems = showElems[p.MirroredElemTypes, p.MirroredElemLabels](0)(x.asInstanceOf[Product])
          if elems.isEmpty then label else label + elems.mkString("(", ", ", ")")

case class Point(x: Int, y: Int) derives Show
case class Pair[T](a: T, b: T) derives Show
sealed trait Shape derives Show
case class Circle(radius: Int) extends Shape
case class Rect(w: Int, h: Int) extends Shape
case object Dot extends Shape
enum Color derives Show:
  case Red, Green
  case Custom(name: String)

object Main:
  def main(args: Array[String]): Unit =
    println(summon[Show[Point]].show(Point(1, 2)))
    println(summon[Show[Pair[String]]].show(Pair("a", "b")))
    println(summon[Show[Shape]].show(Circle(3)))
    println(summon[Show[Shape]].show(Rect(2, 5)))
    println(summon[Show[Shape]].show(Dot))
    println(summon[Show[Color]].show(Color.Green))
    println(summon[Show[Color]].show(Color.Custom("teal")))
    val m = summon[Mirror.ProductOf[Point]]
    println(m.fromProduct((7, 8)))
    println(constValue[m.MirroredLabel])
    println(constValueTuple[m.MirroredElemLabels])
    val s = summon[Mirror.SumOf[Shape]]
    println(s.ordinal(Rect(1, 1)))
    println(constValueTuple[s.MirroredElemLabels])
    println(summon[Mirror.Of[Dot.type]].fromProduct(EmptyTuple))
