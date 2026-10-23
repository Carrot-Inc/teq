/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// The pass's NewValue probe, its fields, values, syntax and derived accessors as scalameta 4.17.4's (structure.txt).
package nodes.Init_ParenType

class Box(x: Int)
object O { val b = new (Box)(1) }

object Marker:
  val marker = 0 // assert: DumpStructure.structure
