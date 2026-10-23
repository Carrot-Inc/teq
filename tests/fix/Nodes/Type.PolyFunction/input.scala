/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.PolyFunction, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_PolyFunction

object Type_PolyFunction:
  val marker = 0 // assert: DumpStructure.structure
  type P = [A] => A => A
  type Q = [A, B] => (A, B) => A
