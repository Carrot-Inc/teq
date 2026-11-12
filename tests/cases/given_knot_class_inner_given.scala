// A plain given's by-name clause answered by an inline given whose class asks for the outer given again: the
// class is made while the outer given's search is open and reads its lazy local.
// dotty makes the lazy implicit a local of the search's owner (`SearchRoot.linkBynameImplicit`) and defines it
// around the outermost search's result (`emitDictionary`), so a class anywhere in the result captures it.
import scala.compiletime.*
trait Show[A]:
  def show(a: A): String
trait Inst[A]:
  def elem: Show[A]
object Inst:
  inline given mk[A]: Inst[A] = new Inst[A]:
    def elem: Show[A] = summonInline[Show[A]]
final case class Rec(n: Int)
object Rec:
  given sub(using i: => Inst[Rec]): Show[Rec] = new Show[Rec]:
    def show(r: Rec): String = if r.n > 3 then "deep" else "sub:" + i.elem.show(Rec(r.n + 1))
@main def run(): Unit =
  println(summon[Show[Rec]].show(Rec(1)))
