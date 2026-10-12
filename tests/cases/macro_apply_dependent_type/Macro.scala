// The type of an application of a dependent method: the parameter replaced by the argument's path
// in the clauses after it and the result, a member selected on it kept a selection on that path
// (`TypeAssigner.safeSubstParam`): `(t: Boxes.ib.T)Boxes.ib.T`, then `Boxes.ib.T`.
import scala.quoted.*
trait Box:
  type T
object Boxes:
  val ib: Box { type T = Int } = new Box { type T = Int }
object Methods:
  def dep(b: Box)(t: b.T): b.T = t
object Mac:
  inline def test: String = ${ impl }
  def impl(using Quotes): Expr[String] =
    import quotes.reflect.*
    val f = Select.unique('{ Methods }.asTerm, "dep")
    val first = Apply(f, List('{ Boxes.ib }.asTerm))
    val last = Apply(first, List('{ 5 }.asTerm))
    val shown = Expr(first.tpe.show + " | " + last.tpe.show + " | ")
    '{ $shown + ${ last.asExprOf[Int] } }
