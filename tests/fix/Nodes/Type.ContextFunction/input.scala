/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.ContextFunction, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_ContextFunction

object Type_ContextFunction:
  val marker = 0 // assert: DumpStructure.structure
  type A = Int ?=> Int
  type B = (Int, String) ?=> Int
