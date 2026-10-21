/*
rules = ["file:../../support/RemoveNamesAtomic.scala"]
*/
// scalafix:off RemoveNamesAtomic
// The removal of a selector's last importee is the whole import's removal (`ImportPatchOps`' `isRemovedImporter` and
// `isRemovedImport`), which `EscapeHatch.filter` checks token by token (`treePatchApply`): the importer's first token is
// suppressed, so the atomic patch goes whole, the import stays and the anchor is used (the review of `b8eb8842a`).
import scala.collection.immutable.{
  // scalafix:on RemoveNamesAtomic
  List
}
object Input
