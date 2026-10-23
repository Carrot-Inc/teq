/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Import, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Import

object Import:
  val marker = 0 // assert: DumpStructure.structure
  import scala.collection.mutable
  import scala.util.{Try, Success}
  val t = Try(1)
