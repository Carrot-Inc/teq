/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// The pass's QualifierValue probe, its fields, values, syntax and derived accessors as scalameta 4.17.4's (structure.txt).
package nodes.Mod_Private_Comment

class O private[/* note */ O] ()

object Marker:
  val marker = 0 // assert: DumpStructure.structure
