// inline match reduces on the scrutinee's static type, its constant value, tuple and case
// class patterns, and guards; the untaken cases are never typed.
import scala.compiletime.{constValue, erasedValue}

sealed trait Shape
case class Circle(radius: Int) extends Shape
case class Rect(w: Int, h: Int) extends Shape
case object Dot extends Shape

object Shapes:
  inline def kind(inline s: Shape): String = inline s match
    case Circle(r) if r > 100 => "big circle " + r
    case Circle(r) => "circle " + r
    case Rect(w, h) => "rect " + (w * h)
    case Dot => "dot"

  inline def typeName(x: Any): String = inline x match
    case _: Int => "Int"
    case _: String => "String"
    case _: List[t] => "List"
    case _ => "other"

  inline def bucket(inline n: Int): String = inline n match
    case 0 => "zero"
    case 1 | 2 => "few"
    case _ => "many"

  inline def swap(pair: (Int, String)): (String, Int) = inline pair match
    case (n, s) => (s, n)

  inline def headOr[T](xs: List[T], inline default: T): T = xs match
    case h :: _ => h
    case Nil => default

  inline def widthOf[T]: Int = inline erasedValue[T] match
    case _: Boolean => 1
    case _: Int => 32
    case _: Long => 64
    case _: String => -1

  inline def label[L]: String = constValue[L].toString

  inline def bound[T](x: T): String = inline x match
    case s: String => "string of " + s.length
    case i: Int => "int " + (i + 1)
    case other => "other " + other

object Main:
  def main(args: Array[String]): Unit =
    import Shapes.*
    println(kind(Circle(200)))
    println(kind(Circle(2)))
    println(kind(Rect(3, 4)))
    println(kind(Dot))
    println(typeName(1) + " " + typeName("s") + " " + typeName(List(1)) + " " + typeName(1.5))
    println(bucket(0) + " " + bucket(2) + " " + bucket(9))
    println(swap((1, "one")))
    println(headOr(List(4, 5), 0) + headOr(Nil, 9))
    println(widthOf[Boolean] + widthOf[Int] + widthOf[Long] + widthOf[String])
    println(label["fast"] + label[7] + label[true])
    println(bound("abc") + ", " + bound(41) + ", " + bound(2.5))
