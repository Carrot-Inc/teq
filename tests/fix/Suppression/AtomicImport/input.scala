/*
rules = ["file:../../support/AddImmutable.scala"]
*/
// scalafix:off AddImmutable
// An atomic import addition is suppressed at the token it is added beside (`EscapeHatch.filter`'s `AtomicPatch`
// branch over `ImportPatchOps`' token patches; the review of `86884ea36`): nothing is added, and the anchor is used.
package probe
import scala.collection.mutable
object Input
