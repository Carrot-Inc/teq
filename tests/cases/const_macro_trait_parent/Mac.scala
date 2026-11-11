// The trait parent's argument of const_trait_parent_argument in a macro's provider unit, which a macro of the later
// round runs: folded there too, before the macro runs its code. From Codex's review of tr/cut-head-tail-c2 6689715658
// (P1-A).
import scala.quoted.*
trait Parent(v: Int):
  val seen: Int = v
class Child(h: Holder) extends Parent(h.x)
class Holder:
  val before: Int = new Child(this).seen
  val x: 42 = 42
object Mac:
  inline def value: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(new Holder().before)
