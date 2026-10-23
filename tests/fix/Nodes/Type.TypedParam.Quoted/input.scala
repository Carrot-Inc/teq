/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// The pass's QuotedName probe, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_TypedParam_Quoted

object Subject { type F = (`a b`: Int) => String; type T = (`c d`: Int, e: String) }

object Marker:
  val marker = 0 // assert: DumpStructure.structure
