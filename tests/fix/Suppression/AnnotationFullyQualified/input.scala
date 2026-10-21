/*
rules = [
  "file:../../support/NoDummy.scala"
  "file:../../support/NoNull.scala"
]

Disable.symbols = [
  "scala.Option.get"
]
*/
package test.escapeHatch

object AnnotationFullyQualified {

  @java.lang.SuppressWarnings(Array("scalafix:NoNull", "scalafix:NoDummy"))
  def aDummy(x: Option[Any]): Unit = {
    val y = null
  }
}
