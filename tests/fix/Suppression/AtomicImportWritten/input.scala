/*
rules = ["file:../../support/AddImmutable.scala"]
*/
/* scalafix:off AddImmutable */ // assert: UnusedScalafixSuppression
// An atomic import addition of an importer the file has written makes no token patch (`treePatchApply`'s dedup), so
// the suppression suppresses nothing and is reported unused (the review of `340567fdd`).
package probe
import scala.collection.immutable
object Input
