// The removal of `List`, the first of the removed run, is its tokens and the comma and whitespace after it
// (`ImportPatchOps`' `remove` and `removeUpToFirstComma`), which stop at the `off` anchor before `Set`: no token of
// the atomic patch is suppressed, so it is applied (`EscapeHatch.filter` checks the token edits applied; the review of
// `4fd7f3293`), and the anchors suppress nothing.
import scala.collection.immutable.{
  /* scalafix:off RemoveListAtomic */ Set, // assert: UnusedScalafixSuppression
  /* scalafix:on RemoveListAtomic */ Map
}
object Input
