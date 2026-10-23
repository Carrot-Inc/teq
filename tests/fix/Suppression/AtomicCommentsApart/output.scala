// The line comment after `Map`'s comma stops `removeUpToFirstComma`'s walk back from `List`, which ends no leading
// run of removed importees, so `List` goes without a comma (`ImportPatchOps.remove`): scalafix 0.14.7 writes the
// comma alone on its line, a file that does not parse, and teq writes dotty's `checkImports` edit there, `List` with
// its comma. The suppression around `Map` suppresses nothing.
import scala.collection.immutable.{
  /* scalafix:off RemoveListAtomic */ Map /* scalafix:on RemoveListAtomic */, // assert: UnusedScalafixSuppression
  Set
}
object Input
