// An implicit conversion whose result holds its type parameter covariantly and whose expected
// type is an open variable of the enclosing call (levsha's `stringToNode[M](s): Node[M]` for a
// `Doc[M]*` argument): the parameter is interpolated as an application's is, `Nothing`, not the
// outer variable's `Any`.
import scala.language.implicitConversions
sealed trait Doc[+M]
final class Node[+M](val s: String) extends Doc[M]
object Dsl:
  def div[M](content: Doc[M]*): Node[M] = Node(content.map { case n: Node[?] => n.s }.mkString("<div>", "", "</div>"))
  implicit def stringToNode[M](value: String): Node[M] = Node(value)
  implicit def miscToNode[M](value: M): Node[M] = Node("misc")
import Dsl.*
object Main:
  def t1(s: String) = div(s)
  def t2(s: String) = div(div(s))
  def main(args: Array[String]): Unit =
    val a: Node[Nothing] = t1("x")
    val b: Node[Nothing] = t2("y")
    println(a.s + b.s)
