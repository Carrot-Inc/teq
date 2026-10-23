/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// The pass's PrivateCtor probe, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Mod_Private_Ctor

class Subject private[Subject] (x: Int)

object Marker:
  val marker = 0 // assert: DumpStructure.structure
