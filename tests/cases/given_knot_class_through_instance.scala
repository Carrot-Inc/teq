// A transparent inline given whose by-name clause is answered by a plain given that asks for it again: the
// inline given's class reads the knot's lazy local through the instance between them.
// dotty makes the lazy implicit a local of the search's owner (`SearchRoot.linkBynameImplicit`) and defines it
// around the outermost search's result (`emitDictionary`), so a class anywhere in the result captures it.
trait Show[A]:
  def show(a: A): String
trait SubShow[A] extends Show[A]
final class Inst[A](val elem: () => Show[A])
object Inst:
  given mk[A](using s: => Show[A]): Inst[A] = new Inst(() => s)
final case class Rec(n: Int)
object Rec:
  transparent inline given sub(using i: => Inst[Rec]): SubShow[Rec] = new SubShow[Rec]:
    def show(r: Rec): String = if r.n > 3 then "deep" else "sub:" + i.elem().show(Rec(r.n + 1))
@main def run(): Unit =
  println(summon[SubShow[Rec]].show(Rec(1)))
