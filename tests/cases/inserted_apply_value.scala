// The `apply` of a value applied to arguments: a method taking them, or a function value's own
// `apply`; an `apply` member that is a value is applied when it is written (dotty's
// `Typer.tryInsertApplyOrImplicit` inserts one `apply` for the arguments, and an explicit
// `.apply` selection is none, nor one given explicit type arguments).
class Ops(val apply: Int => Int)
class M { def apply(n: Int): Int = n * 3 }
object P:
  def apply(n: Int): Int = n + 100
class Builder[S] { def apply(name: String): String = name + "!" }
object Lens:
  def apply[S]: Builder[S] = new Builder[S]
@main def run(): Unit =
  val g: Int => Int = _ + 5
  println(g(1))
  println(M()(2))
  println(P(1))
  val ops = Ops(_ * 7)
  println(ops.apply(2))
  println(ops.apply.apply(3))
  val fs = List[Int => Int](_ + 1)
  println(fs(0)(4))
  println(Lens[Int]("n"))
