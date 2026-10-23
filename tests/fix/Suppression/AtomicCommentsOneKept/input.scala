/*
rules = ["file:../../support/RemoveListAtomic.scala"]
*/
// One selector retained: the removal of `List` with its comma, and the braces with the whitespace inside them
// (`curlyBraceRemoves`), reach no anchor, so the atomic patch is applied and the anchors suppress nothing.
import scala.collection.immutable.{
  List,
  /* scalafix:off RemoveListAtomic */ Set /* scalafix:on RemoveListAtomic */ // assert: UnusedScalafixSuppression
}
object Input
