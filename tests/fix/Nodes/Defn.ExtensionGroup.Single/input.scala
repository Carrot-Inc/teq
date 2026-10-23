/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// The pass's SingleExtension probe, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Defn_ExtensionGroup_Single

object Subject:
  extension (x: Int)
    def twice: Int = x + x

object Marker:
  val marker = 0 // assert: DumpStructure.structure
