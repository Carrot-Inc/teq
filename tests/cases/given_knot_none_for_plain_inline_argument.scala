// No knot here: a plain inline given found for an argument of a transparent one is expanded in dotty's inlining
// phase, where no search is open, so the request its body makes is searched afresh and finds `plain`.
import scala.compiletime.*

trait Show[A]:
  def show(a: A): String
trait SubShow[A] extends Show[A]

final class Inst[A](val elem: () => Show[A])
object Inst:
  inline given mk[A]: Inst[A] = new Inst(() => summonInline[Show[A]])

final case class Rec(n: Int)
final class Sub(i: () => Inst[Rec]) extends SubShow[Rec]:
  def show(r: Rec): String = if r.n > 3 then "deep" else "sub:" + i().elem().show(Rec(r.n + 1))
object Rec:
  given plain: Show[Rec] = r => "plain" + r.n
  transparent inline given sub(using i: => Inst[Rec]): SubShow[Rec] = new Sub(() => i)

@main def run(): Unit =
  println(summon[SubShow[Rec]].show(Rec(1)))
