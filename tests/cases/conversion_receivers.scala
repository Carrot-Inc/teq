// A conversion is called on what it was found through: a val an import opens that an object
// inherits from a trait (`import O.dsl.*`), a Scala 2 implicit `Conversion` value of such a val,
// and a given `Conversion` a companion inherits from a trait (scalac: the lines below).
import scala.language.implicitConversions

class Dsl(tag: String):
  implicit def asString(i: Int): String = s"$tag:$i"
  implicit val toFlag: Conversion[Boolean, String] = b => s"$tag flag $b"
  def name: String = tag
trait Holder:
  val dsl: Dsl = new Dsl("held")
object O extends Holder

class Box(val v: Int)
object Box extends BoxConv
trait BoxConv:
  given Conversion[Box, String] = b => s"box ${b.v}"

@main def main(): Unit =
  import O.dsl.*
  val s: String = 3
  println(s)
  println(name)
  val f: String = true
  println(f)
  val b: String = Box(1)
  println(b)
