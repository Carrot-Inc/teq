/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Pat.Assign, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Pat_Assign

object Pat_Assign:
  val marker = 0 // assert: DumpStructure.structure
  val p = (a = 1, b = 2)
  val r = p match
    case (a = x, b = y) => x + y
