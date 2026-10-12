// A trait's members reached through an object that extends it are seen from the object, as
// dotty's TermRef(dsl.type, m) keeps the prefix: an inner object, an inner class's instance, a
// given object, an extension called on the object, through an import of it or as a conversion.
import scala.language.implicitConversions

trait Writer[V]
given Writer[Int] = new Writer[Int] {}
trait Producer
class Doc(val tag: String) extends Producer

trait Lang:
  class Expr(val n: Int)
  def mk(n: Int): Expr = new Expr(n)
  object inner:
    def apply(e: Expr): Int = e.n
    def make: Expr = new Expr(3)
  class Box:
    def apply(e: Expr): Int = e.n + 100
  val box = new Box
  given conv: Conversion[Expr, Int] with
    def apply(e: Expr): Int = e.n * 2
  sealed trait Expression[V]:
    def value: V
  final class SimpleExpression[V](val value: V) extends Expression[V]
  given toDoc[V](using Writer[V]): Conversion[Expression[V], Doc] = e => Doc(s"doc ${e.value}")
  def bdoc(elements: Producer*): String = elements.map { case d: Doc => d.tag; case _ => "?" }.mkString(", ")
  extension (field: String)
    def in(v: Int): SimpleExpression[Int] = SimpleExpression(v)
    def at(v: Int): Expr = new Expr(v)

object dsl extends Lang
class Holder extends Lang

@main def main(): Unit =
  println(dsl.inner(dsl.mk(7)))
  val made: dsl.Expr = dsl.inner.make
  println(dsl.box(made))
  println(dsl.conv(dsl.mk(4)))
  val e: dsl.Expr = dsl.at("f")(5)
  println(e.n)
  val h = new Holder
  val g: h.Expr = h.at("g")(6)
  println(h.inner(g))
  locally {
    import dsl.{*, given}
    val viaImport: dsl.Expr = "x".at(8)
    println(inner(viaImport))
    val doubled: Int = mk(9)
    println(doubled)
    println(bdoc(SimpleExpression(1), "f".in(2)))
  }
