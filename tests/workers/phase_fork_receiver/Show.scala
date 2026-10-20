// jars: sourcecode
import scala.compiletime.{constValue, erasedValue, summonInline}
import scala.deriving.Mirror

trait Show[T]:
  def show(t: T): String

object Show:
  given Show[Int] with
    def show(t: Int): String = t.toString

  inline def elems[E <: Tuple](p: Product, n: Int): List[String] = inline erasedValue[E] match
    case _: EmptyTuple => Nil
    case _: (e *: es) => summonInline[Show[e]].show(p.productElement(n).asInstanceOf[e]) :: elems[es](p, n + 1)

  inline def derived[T](using m: Mirror.ProductOf[T]): Show[T] = new Show[T]:
    def show(t: T): String = constValue[m.MirroredLabel] + elems[m.MirroredElemTypes](t.asInstanceOf[Product], 0).mkString("(", ",", ")")
