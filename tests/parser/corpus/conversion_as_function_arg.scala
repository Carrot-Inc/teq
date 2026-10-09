// An implicit conversion method is an implicit value of the function type it converts along:
// it fills an implicit parameter `f: A => Node`, and the body of an eta-expanded method is
// converted to the result type the expected function type has (scalac: "node(a)", "node(b)",
// "node(c)", "node(d)", "node(e)", "node(none)").
import scala.language.implicitConversions

final case class Node(s: String)
object Implicits:
  implicit def nodeFromString(s: String): Node = Node(s)
  implicit def nodeFromOption[A](o: Option[A])(implicit f: A => Node): Node = o.map(f).getOrElse(Node("none"))
import Implicits.*

def show(a: String): String = a
def render[A](a: A)(implicit f: A => Node): String = s"node(${f(a).s})"

@main def main(): Unit =
  val n: Node = Option("a")
  println(s"node(${n.s})")
  println(s"node(${Option("b").map[Node](show).get.s})")
  val g: String => Node = show
  println(s"node(${g("c").s})")
  println(render("d"))
  println(render("e"))
  val none: Node = Option.empty[String]
  println(s"node(${none.s})")
