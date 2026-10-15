import scala.annotation.targetName

// A value and a method of one name that takes parameters are two members, never one overriding
// the other (dotty's `Signature.matchDegree`: their parameters are not consistent, `NoMatch`), as
// tyrian's `Html` object takes attribute values and tag methods of one name from two traits and
// declares methods beside inherited values: inherited from unrelated traits, a value beside a
// method below it, a method beside a value below it, and values of distinct target names beside
// methods, each chosen by its arguments or by the type expected.
trait Tags:
  def cite[M](children: List[M]): String = "tag " + children.size
  @targetName("cite-repeated") def cite[M](children: M*): String = "tags " + children.size

trait Attributes:
  @targetName("cite-String") val cite: String = "attr"
  @targetName("style-String") val style: String = "style-string"
  @targetName("style-Int") val style: Int = 7

object Html extends Tags with Attributes:
  def style(name: String, value: String): String = s"$name:$value"
  @targetName("style_pair") def style(pair: (String, String)): String = pair._1

class Base:
  val x: Int = 1
  def y(s: String): String = "y " + s
class Derived extends Base:
  def x(s: String): String = s + x
  val y: Int = 5

trait T:
  val z: String = "t"
trait U:
  def z(n: Int): Int = n * 2
object C extends T with U

@main def run(): Unit =
  val a: String = Html.cite
  println(a)
  println(Html.cite(List(1, 2)))
  println(Html.cite(1, 2, 3))
  println(Html.style("a", "b"))
  println(Html.style("c" -> "d"))
  val s: String = Html.style
  val i: Int = Html.style
  println(s"$s $i")
  val d = Derived()
  println(d.x("x"))
  println(d.y("s") + " " + (d.y + 1))
  println(C.z + C.z(3))
