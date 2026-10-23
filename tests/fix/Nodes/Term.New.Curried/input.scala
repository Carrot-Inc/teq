/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// The pass's CurriedNew probe, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_New_Curried

class Box(x: Int)(y: Int)
object Subject { val value: Box = new Box(1)(2) }

object Marker:
  val marker = 0 // assert: DumpStructure.structure
