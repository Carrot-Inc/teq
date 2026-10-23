/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Defn.EnumCase, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Defn_EnumCase

object Defn_EnumCase:
  val marker = 0 // assert: DumpStructure.structure
  enum E:
    case A(x: Int)
    case B extends E
    case C[T](t: T) extends E
