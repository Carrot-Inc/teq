// A receiver of a context function type is applied to the givens in scope before a member other than
// its apply is selected (dotty's adaptNoArgsOther against a selection), as scalachess's
// ByColor.contains(a: A): Eq[A] ?=> Boolean is in lila's players.contains(player).option(...).
trait Eq[A]:
  def eqv(x: A, y: A): Boolean
given Eq[Int] = (x, y) => x == y

case class ByColor[A](white: A, black: A):
  def contains(a: A): Eq[A] ?=> Boolean = summon[Eq[A]].eqv(white, a) || summon[Eq[A]].eqv(black, a)

extension (b: Boolean) def option[A](a: => A): Option[A] = if b then Some(a) else None

type Ctx[A] = String ?=> A
def greet(name: String): Ctx[List[String]] = List(summon[String], name)

@main def run(): Unit =
  val p = ByColor(1, 2)
  println(p.contains(2).option("yes"))
  println(p.contains(3).option("no"))
  given String = "hello"
  println(greet("ann").map(_.length))
  println(greet("bob").size)
