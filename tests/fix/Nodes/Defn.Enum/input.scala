/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Defn.Enum, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Defn_Enum

object Defn_Enum:
  val marker = 0 // assert: DumpStructure.structure
  enum Color:
    case Red, Green
  enum Opt[+A]:
    case Some(a: A)
    case None
