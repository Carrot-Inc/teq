// A plain inline given whose by-name clause is answered by an inline given that asks for it again with
// `summonInline`: the lazy local is made inside the inner expansion and read by the outer given's class.
// dotty makes the lazy implicit a local of the search's owner (`SearchRoot.linkBynameImplicit`) and defines it
// around the outermost search's result (`emitDictionary`), so a class anywhere in the result captures it.
import scala.compiletime.*
trait Show[A]:
  def show(a: A): String
final class Inst[A](val elem: () => Show[A])
object Inst:
  inline given mk[A]: Inst[A] = new Inst(() => summonInline[Show[A]])
final case class Rec(n: Int)
object Rec:
  inline given sub(using i: => Inst[Rec]): Show[Rec] = new Show[Rec]:
    def show(r: Rec): String = if r.n > 3 then "deep" else "sub:" + i.elem().show(Rec(r.n + 1))
inline def get: Show[Rec] = summonInline[Show[Rec]]
@main def run(): Unit =
  println(get.show(Rec(1)))
