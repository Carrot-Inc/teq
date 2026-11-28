// An expected result type can change the overload chosen at the last written argument list.
// With a second list, the more specific first overload supplies the value applied to 1.
import scala.language.implicitConversions

object O:
  def m(x: Int): String = "member"
  def m(x: Any)(i: Int): Int = 42

  def checkThis(): Unit =
    val r: Int = this.m(0)(1)
    println(s"this $r")

object ByName:
  def m(x: Int): String = "member"
  def m(x: Any)(i: => Int): Int = 42

class Box
object Box:
  implicit def toFunction(b: Box): Int => Int = i => i + 100

object Conv:
  def m(x: Int): Box = new Box
  def m(x: Any)(i: Int): Int = 42

object Fun:
  def m(x: Int): Int => Int = i => i + 100
  def m(x: Any)(i: Int): Long = 42L

trait Parent:
  def m(x: Int): String = "member"
  def m(x: Any)(i: Int): Int = 42

object Child extends Parent:
  def checkSuper(): Unit =
    val r: Int = super.m(0)(1)
    println(s"super $r")

object Control:
  def f(x: Int): String = "s"
  def f(x: Any): Int = 42

  def check(): Unit =
    val r: Int = f(0)
    println(s"control $r")

@main def run(): Unit =
  val r: Int = O.m(0)(1)
  println(s"apply $r")
  val byName: Int = ByName.m(0)(1)
  println(s"byname $byName")
  val conv: Int = Conv.m(0)(1)
  println(s"conv $conv")
  val fun: Long = Fun.m(0)(1)
  println(s"fun $fun")

  O.checkThis()
  Child.checkSuper()
  val o = O
  val qual: Int = o.m(0)(1)
  println(s"qual $qual")
  import O.*
  val imported: Int = m(0)(1)
  println(s"import $imported")
  Control.check()
