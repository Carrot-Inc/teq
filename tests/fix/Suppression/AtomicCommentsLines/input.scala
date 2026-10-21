/*
rules = ["file:../../support/RemoveListAtomic.scala"]
*/
// AtomicCommentsBetween with line comments: the edit removing `List` reaches over the `off` line before `Set`.
import scala.collection.immutable.{
  List,
  // scalafix:off RemoveListAtomic
  Set,
  // scalafix:on RemoveListAtomic
  Map
}
object Input
