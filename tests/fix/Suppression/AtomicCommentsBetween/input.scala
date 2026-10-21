/*
rules = ["file:../../support/RemoveListAtomic.scala"]
*/
// The removal of `List` is applied as dotty's import edit, which reaches to the next selector and so over the `off`
// anchor before `Set`: the atomic patch has a suppressed token in its edit and goes whole, so that no anchor is deleted
// (`EscapeHatch.filter` checks the token edits applied; the review of `4fd7f3293`).
import scala.collection.immutable.{
  List,
  /* scalafix:off RemoveListAtomic */ Set,
  /* scalafix:on RemoveListAtomic */ Map
}
object Input
