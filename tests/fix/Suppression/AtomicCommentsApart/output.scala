// AtomicComments' control: the edit removing `List` reaches to `Set`, no anchor in it, so it is applied, and the anchor
// around `Map` suppresses nothing.
import scala.collection.immutable.{
  /* scalafix:off RemoveListAtomic */ Map /* scalafix:on RemoveListAtomic */, // assert: UnusedScalafixSuppression
  Set
}
object Input
