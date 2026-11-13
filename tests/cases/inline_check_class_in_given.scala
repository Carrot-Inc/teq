// An inline member of a class made in a given's initialiser summons the type the given is of: the given being
// defined is a candidate there, since a class ends the reach of "the given under definition" (scalac leaves a given
// out of a search in its own right-hand side only up to the first enclosing class). So the member finds the nearer
// given whether the class's own walk checks it first (the member written before its caller) or a sibling's body asks
// for it first, with or without another given farther out.
trait Stamp:
  def name: String

final case class Tree(label: String, kids: List[Tree])

trait Render[T]:
  def render(t: T): String

object Farther:
  given far: Stamp with
    def name: String = "far"

object MemberFirst:
  import Farther.given
  def run: String =
    given near: Stamp = new Stamp:
      transparent inline def pick: Stamp = summon[Stamp]
      def name: String = if pick eq this then "near" else pick.name
    near.name

object CallerFirst:
  import Farther.given
  def run: String =
    given near: Stamp = new Stamp:
      def name: String = if pick eq this then "near" else pick.name
      transparent inline def pick: Stamp = summon[Stamp]
    near.name

object PlainInferred:
  import Farther.given
  def run: String =
    given near: Stamp = new Stamp:
      def name: String = if pick eq this then "near" else pick.name
      inline def pick = summon[Stamp]
    near.name

object Alone:
  given trees: Render[Tree] = new Render[Tree]:
    def render(t: Tree): String = t.label + t.kids.map(k => one(k)).mkString("(", ",", ")")
    transparent inline def one(t: Tree): String = summon[Render[Tree]].render(t)

@main def run(): Unit =
  println(MemberFirst.run)
  println(CallerFirst.run)
  println(PlainInferred.run)
  println(summon[Render[Tree]](using Alone.trees).render(Tree("a", List(Tree("b", Nil), Tree("c", List(Tree("d", Nil)))))))
