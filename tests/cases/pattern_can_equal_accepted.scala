// Patterns compared by `==` that the comparison's evidence allows, under strictEquality: an enum deriving
// CanEqual, literals and vals of a primitive or String, None and Nil, an object of the scrutinee's own type, a
// numeric literal of another width, a class that declares `==` for the scrutinee (accepted; what it matches is a
// row of its own), given evidence written in the pattern's order, and a GADT case with its evidence.
import scala.language.strictEquality
import scala.language.experimental.strictEqualityPatternMatching

enum Shape derives CanEqual:
  case Sq, Ci
case object Solo
class A
class B { def ==(a: A): Boolean = true }
class C
class D
sealed trait Tag[T]
case object IntTag extends Tag[Int]
given CanEqual[Tag[Int], Tag[Int]] = CanEqual.derived
given CanEqual[D, C] = CanEqual.derived

trait Marker
object Numbers:
  opaque type N <: Int = Int
  def n(i: Int): N = i
trait Element
case object Elem extends Element
trait Members:
  type X <: Element
  type S <: String
  def adt(x: X): String = x match { case Elem => "elem"; case _ => "other" }
  def list(x: List[S], b: List[String]): String = x match { case `b` => "same"; case _ => "other" }

object Main:
  def boundedText[T <: String](x: T) = x match { case "a" => "a"; case _ => "other" }
  def boundedNumber[T <: Int](x: T) = x match { case 1 => "one"; case _ => "other" }
  def opaqueNumber(x: Numbers.N) = x match { case 1 => "one"; case _ => "other" }
  def bounded[T <: String](x: T): Boolean = x == "a"
  val Pi = 3
  val Name = "n"
  val b: B = new B
  val d: D = new D
  def shape(s: Shape) = s match { case Shape.Sq => "sq"; case Shape.Ci => "ci" }
  def int(n: Int) = n match { case 1 => "one"; case Pi => "pi"; case _ => "other" }
  def text(s: String) = s match { case "a" => "a"; case Name => "name"; case _ => "other" }
  def option(o: Option[Int]) = o match { case None => "none"; case Some(n) => "some " + n }
  def list(l: List[Int]) = l match { case Nil => "nil"; case _ => "cons" }
  def solo(s: Solo.type) = s match { case Solo => "solo" }
  def long(l: Long) = l match { case 1 => "one"; case _ => "other" }
  def declared(a: A) = a match { case `b` => "b"; case _ => "other" }
  def evidence(c: C) = c match { case `d` => "d"; case _ => "other" }
  def gadt[T](t: Tag[T], x: T): Int = t match { case IntTag => val n: Int = x; n + 1 }

  def main(args: Array[String]): Unit =
    println(List(shape(Shape.Sq), int(3), text("n"), option(None), list(Nil), solo(Solo), long(1L), evidence(new C), gadt(IntTag, 41), boundedText("a"), boundedNumber(1), opaqueNumber(Numbers.n(1)), bounded("a")).mkString(" "))
    val m = new Members { type X = Element; type S = String }
    println(m.adt(Elem) + " " + m.list(List("a"), List("a")))
