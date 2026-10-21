/*
rules = ["file:../../support/RemoveListAtomic.scala"]
*/
// One selector retained: the edit as applied rewrites the statement (`import scala.collection.immutable.Set`), over both
// anchors, so the atomic patch goes whole and the file stays as it is.
import scala.collection.immutable.{
  List,
  /* scalafix:off RemoveListAtomic */ Set /* scalafix:on RemoveListAtomic */
}
object Input
