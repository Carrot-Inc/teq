// A recursive instance tied through an inline given: the by-name parameter of `Show[Rec]`'s given is answered by
// the inline `mk`, and the request that closes the circle (`Show[Option[Rec]]` again) is made by `mk`'s body, a
// `summonInline`. dotty types a candidate under `searchHistory.nest(cand, pt)` (`ImplicitSearch.tryImplicit`), the
// open search by-name when the request it answers is, and that is the context in which `Typer.adapt` expands an
// inline candidate: `recursiveRef` finds the first request below it and ties the two (Implicits.scala). scalac 3.8.4
// prints `1>2>-`.
import scala.compiletime.*

trait Show[A]:
  def show(a: A): String

final class Inst[A](val elem: () => Show[A])
object Inst:
  inline given mk[A]: Inst[A] = new Inst(() => summonInline[Show[A]])

final case class Rec(n: Int, next: Option[Rec])

object Show:
  given Show[Int] = _.toString
  given opt[A](using s: Show[A]): Show[Option[A]] = o => o.fold("-")(s.show)
object Rec:
  given (using i: => Inst[Option[Rec]]): Show[Rec] = r => s"${r.n}>${i.elem().show(r.next)}"

@main def run(): Unit =
  println(summon[Show[Rec]].show(Rec(1, Some(Rec(2, None)))))
