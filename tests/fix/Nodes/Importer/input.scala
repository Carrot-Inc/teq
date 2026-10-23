/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Importer, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Importer

object Importer:
  val marker = 0 // assert: DumpStructure.structure
  import scala.collection.mutable.{Map => MMap, Set as MSet}
  val m = MMap(1 -> 2)
