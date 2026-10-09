// A conversion method in an inner scope is the implicit function value there, before a given
// `Conversion` of an outer one, which is taken outside (scalac: "def x", "given y").
import scala.language.implicitConversions
final case class Node(s: String)
given Conversion[String, Node] = s => Node("given " + s)
def render(a: String)(implicit f: String => Node): String = f(a).s
object Inner:
  implicit def nodeFromString(s: String): Node = Node("def " + s)
  def run(): Unit = println(render("x"))
@main def main(): Unit =
  Inner.run()
  println(render("y"))
