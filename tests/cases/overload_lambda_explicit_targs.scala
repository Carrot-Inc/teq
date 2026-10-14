// Explicit type arguments are every alternative's before the arguments are typed ahead: with them
// both alternatives take a function of a String, so `Move(_)` is typed once and only `A => B` fits its
// `String => Move` (dotty's `resolveMapped` of a `PolyProto`, then `pretypeArgs` and `narrowByTrees`).
import scala.annotation.targetName
case class Node[A](value: A)
object Tree:
  def build[A, B](s: Seq[A], f: A => B): Option[Node[B]] = s.headOption.map(a => Node(f(a)))
  @targetName("buildNodes")
  def build[A, B](s: Seq[A], f: A => Node[B]): Option[Node[B]] = s.headOption.map(f)
case class Move(s: String)
@main def run(): Unit =
  println(Tree.build[String, Move](List("x"), Move(_)))
  println(Tree.build[String, Move](List("y"), s => Node(Move(s + "!"))))
  println(Tree.build[String, Int](List("zz"), _.length))
