/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Defn.Object, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Defn_Object

object Defn_Object:
  val marker = 0 // assert: DumpStructure.structure
  object A
  case object B
  object C extends Serializable:
    val x = 1
