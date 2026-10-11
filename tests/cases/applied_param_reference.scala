// A higher-kinded parameter applied, `TRAV[E]` of a `TRAV[e] <: Iterable[e]`, is its bound applied
// (dotty's `AppliedType.superType`): a reference, so it conforms to `AnyRef` and to a structural
// `AnyRef { def isEmpty: Boolean }`. Of two givens, one over such a parameter and one over a
// `T <: AnyRef { def isEmpty: Boolean }`, the first is then the more specific, the other's bound held
// at the first's result (`Applications.isAsGood`), as ScalaTest's `Emptiness` declares them.

trait Emptiness[-T]:
  def name: String

object Emptiness:
  implicit def ofIterable[E, TRAV[e] <: Iterable[e]]: Emptiness[TRAV[E]] =
    new Emptiness[TRAV[E]] { def name = "iterable" }
  implicit def ofParameterless[T <: AnyRef { def isEmpty: Boolean }]: Emptiness[T] =
    new Emptiness[T] { def name = "parameterless" }

final case class Period(from: Int, to: Int)

def which[T](t: T)(using e: Emptiness[T]): String = e.name
def asReference[E, TRAV[e] <: Iterable[e]](x: TRAV[E]): AnyRef = x
def asStructural[E, TRAV[e] <: Iterable[e]](x: TRAV[E]): AnyRef { def isEmpty: Boolean } = x

@main def run(): Unit =
  println(which(Set(Period(1, 2))))
  println(which(List(1, 2)))
  println(asReference(Vector(1)) eq null)
  println(asStructural(List(1)) eq null)
