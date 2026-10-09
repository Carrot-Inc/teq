package probe.pi
import scala.compiletime.{erasedValue, summonFrom}
import scala.deriving.Mirror
trait Eq[A]:
  def eqv(a: A, b: A): Boolean
object Eq:
  inline def derived[A](using m: Mirror.Of[A]): Eq[A] =
    inline m match
      case s: Mirror.SumOf[A] =>
        val cases = caseInstances[s.MirroredElemTypes]
        (x, y) => s.ordinal(x) == s.ordinal(y) && cases(s.ordinal(x)).eqv(x, y)
      case p: Mirror.ProductOf[A] => (x, y) => true
  inline def caseInstances[T <: Tuple]: List[Eq[Any]] =
    inline erasedValue[T] match
      case _: EmptyTuple => Nil
      case _: (a *: b *: c *: d *: t) => one[a] :: one[b] :: one[c] :: one[d] :: caseInstances[t]
      case _: (h *: t) => one[h] :: caseInstances[t]
  inline def one[T]: Eq[Any] = summonFrom {
    case e: Eq[T] => e.asInstanceOf[Eq[Any]]
    case m: Mirror.Of[T] => derived[T](using m).asInstanceOf[Eq[Any]]
  }
enum Big derives Eq:
  case C0, C1, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, C12, C13, C14, C15, C16, C17, C18, C19, C20, C21, C22, C23, C24, C25, C26, C27, C28, C29, C30, C31, C32, C33, C34, C35, C36, C37, C38, C39, C40, C41, C42
object Main:
  def main(args: Array[String]): Unit =
    println(summon[Eq[Big]].eqv(Big.C3, Big.C3))
    println(summon[Eq[Big]].eqv(Big.C3, Big.C40))
