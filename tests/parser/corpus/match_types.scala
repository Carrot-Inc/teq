// Match types: reduction by subtyping, disjointness through single inheritance, final classes,
// literals and sealed hierarchies, recursion through the reduced type, a declared bound, and
// a stuck match type used as an abstract type in generic code.
import scala.compiletime.ops.int.*

type Elem[X] = X match
  case String => Char
  case Array[t] => t
  case List[t] => t
  case Option[t] => Elem[t]

type LeafElem[X] = X match
  case String => Char
  case Array[t] => LeafElem[t]
  case List[t] => LeafElem[t]
  case AnyVal => X

type Code[X] = X match
  case 1 => "one"
  case 2 => "two"
  case Int => "many"

sealed trait Shape
final class Circle(val r: Double) extends Shape
final class Square(val side: Double) extends Shape
case object Dot extends Shape

type Area[S] = S match
  case Circle => Double
  case Square => Double
  case Dot.type => Int

type Length[X <: Tuple] <: Int = X match
  case EmptyTuple => 0
  case x *: xs => S[Length[xs]]

type Reverse[X <: Tuple] <: Tuple = X match
  case EmptyTuple => EmptyTuple
  case x *: xs => Tuple.Concat[Reverse[xs], x *: EmptyTuple]

type Doubled[N <: Int] = N * 2

type Pick[X, Y] = (X, Y) match
  case (Int, Int) => "ints"
  case (String, _) => "string first"
  case _ => "mixed"

type Unwrap[X] = X match
  case Option[t] => t
  case _ => X

object Main:
  def elemOf[X](x: X): Elem[X] = x match
    case s: String => s.charAt(0).asInstanceOf[Elem[X]]
    case l: List[t] => l.head.asInstanceOf[Elem[X]]
    case o: Option[t] => elemOf(o.get).asInstanceOf[Elem[X]]
    case other => other.asInstanceOf[Elem[X]]

  def main(args: Array[String]): Unit =
    val a: Elem[String] = 'c'
    val b: Elem[List[Int]] = 3
    val c: Elem[Option[List[Boolean]]] = true
    val d: LeafElem[List[Array[String]]] = 'x'
    val e: LeafElem[Int] = 4
    val f: Code[1] = "one"
    val g: Code[2] = "two"
    val h: Code[7] = "many"
    val i: Area[Circle] = 3.5
    val j: Area[Dot.type] = 1
    val k: Length[(Int, String, Boolean)] = 3
    val l: Reverse[(Int, String, Boolean)] = (true, "s", 1)
    val m: Doubled[21] = 42
    val n: Pick[Int, Int] = "ints"
    val o: Pick[String, Boolean] = "string first"
    val p: Pick[Boolean, Int] = "mixed"
    val q: Unwrap[Option[String]] = "u"
    val r: Unwrap[Long] = 9L
    val s: Elem[String] = elemOf("hello")
    val t: Elem[List[Int]] = elemOf(List(7, 8))
    println(List(a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t).mkString(" "))
