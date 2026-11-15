// A derivation whose inline method conses `summonInline[Show[h]]` onto its own recursive call: the givens are made in
// the order of the case class's fields, the left operand of `::` (a call still to be inlined, under a written cast)
// bound ahead of the right one as `Typer.typedInfixOp` lifts it and kept bound when the inlining makes it a given's
// read. A whole build of the two files made the last field's given first.
import scala.compiletime.{erasedValue, summonInline}
import scala.deriving.Mirror
trait Show[T] { def show(t: T): String }
object Show:
  inline def elems[Ts <: Tuple]: List[Show[Any]] = inline erasedValue[Ts] match
    case _: EmptyTuple => Nil
    case _: (h *: t) => summonInline[Show[h]].asInstanceOf[Show[Any]] :: elems[t]
  inline def derived[T](using m: Mirror.ProductOf[T]): Show[T] =
    val shows = elems[m.MirroredElemTypes]
    new Show[T]:
      def show(t: T): String = t.asInstanceOf[Product].productIterator.zip(shows.iterator).map((v, s) => s.show(v)).mkString("(", ",", ")")
