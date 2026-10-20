// lila: RelayRoundRepo: idsByTourOrdered(tourId).map(StudyId.from)
// `from[M[_], B](using SameRuntime[B, Impl])(f: M[B])` has its using clause first.
// scalac (Typer.adaptNoArgs: constrainResult before adaptNoArgsImplicitMethod) first
// constrains B from what follows (the expected function type / the next argument), then
// searches SameRuntime[RoundId, String], found in RoundId's implicit scope.
trait SameRuntime[A, T] { def apply(a: A): T }
abstract class Wrapper[New, Impl]:
  def apply(i: Impl): New = i.asInstanceOf[New]
  def from[M[_], B](using SameRuntime[B, Impl])(f: M[B]): M[New] = f.asInstanceOf[M[New]]
  given SameRuntime[New, Impl] = _.asInstanceOf[Impl]
object ids:
  opaque type RoundId = String
  object RoundId extends Wrapper[RoundId, String]
  opaque type StudyId = String
  object StudyId extends Wrapper[StudyId, String]
import ids.*

@main def run() =
  val rounds: Option[List[RoundId]] = Some(List(RoundId("r1"), RoundId("r2")))
  val studies: Option[List[StudyId]] = rounds.map(StudyId.from)
  println(studies)
