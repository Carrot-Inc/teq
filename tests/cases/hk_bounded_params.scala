// Bounds on the parameters of a higher-kinded type parameter (`E[+x <: Node] <: Event[x]`),
// in a class, an implicit class and a type lambda.
trait Node:
  def tag: String
class Div extends Node:
  def tag = "div"
class Span extends Node:
  def tag = "span"

trait Event[+N <: Node]:
  def target: N
class KeyEvent[+N <: Node](val target: N) extends Event[N]
class MouseEvent[+N <: Node](val target: N) extends Event[N]

class Attr[E[+x <: Node] <: Event[x]](val name: String):
  def fire[N <: Node](e: E[N]): String = s"$name on ${e.target.tag}"

object Main:
  implicit class AttrOps[E[+x <: Node] <: Event[x]](val attr: Attr[E]):
    def describe: String = s"attr ${attr.name}"

  type Handler[E[+x <: Node] <: Event[x], N <: Node] = E[N] => String
  type Boxed = [x <: Node] =>> KeyEvent[x]
  def handle[E[+x <: Node] <: Event[x], N <: Node](e: E[N], h: Handler[E, N]): String = h(e)

  def main(args: Array[String]): Unit =
    val onKey = new Attr[KeyEvent]("onKey")
    val onClick = new Attr[MouseEvent]("onClick")
    println(onKey.fire(new KeyEvent(new Div)))
    println(onClick.fire(new MouseEvent(new Span)))
    println(onKey.describe)
    println(handle(new KeyEvent(new Span), (e: KeyEvent[Span]) => "handled " + e.target.tag))
    val b: Boxed[Div] = new KeyEvent(new Div)
    println(b.target.tag)
