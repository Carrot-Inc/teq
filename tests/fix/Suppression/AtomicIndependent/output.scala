// Two atomic patches of one statement, each tested on its own (`EscapeHatch.filter`): `Vector`'s edit is suppressed
// and goes, `List`'s is not and is applied (the review of `70fa37159`).
import scala.collection.immutable.{
  Set,
  /* scalafix:off RemoveListVectorAtomic */ Vector,
  /* scalafix:on RemoveListVectorAtomic */ Map,
  Seq
}
object Input
