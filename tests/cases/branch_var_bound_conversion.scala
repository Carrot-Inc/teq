// The branches of an `if` or a `match` expected to be a type variable whose bound the expected
// result fixes (`getOrElse[B >: A]` with the result expected to be a `Node`) are converted into
// the bound where they do not conform; without an expected result they join to their union
// (scalac: "Node(x)", "Node(z)", "Unmounted(x)").
import scala.language.implicitConversions
final case class Node(s: String)
final case class Unmounted(s: String)
implicit def nodeFromUnmounted(u: Unmounted): Node = Node(u.s)

@main def main(): Unit =
  val flag = true
  val r: Node = Option.empty[Node].getOrElse(if flag then Unmounted("x") else Node("y"))
  println(r)
  val m: Node = Option.empty[Node].getOrElse(flag match
    case true => Unmounted("z")
    case false => Node("w"))
  println(m)
  val u = Option.empty[Node].getOrElse(if flag then Unmounted("x") else Node("y"))
  println(u)
