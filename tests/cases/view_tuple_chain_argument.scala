// Implicits.ImplicitRefs.filterMatching's view arm (viewCandidateKind's methodCandidateKind): a conversion
// is eligible when the argument conforms to its parameter by `relaxed_<:<`, which compares a pair chain
// with a TupleN parameter through its pairs (TypeComparer: TupleN formal against `*:`).
import scala.language.implicitConversions

final class Wrapped(val value: Int)
object Wrapped:
  implicit def fromTuple(x: (Int, String)): Wrapped = new Wrapped(x._1)

def convert(x: Int *: String *: EmptyTuple): Wrapped = x

@main def run(): Unit =
  println(convert((7, "a")).value)
  val chain: Int *: String *: EmptyTuple = (8, "b")
  val w: Wrapped = chain
  println(w.value)
