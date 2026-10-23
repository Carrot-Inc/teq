/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Defn.GivenAlias, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Defn_GivenAlias

object Defn_GivenAlias:
  val marker = 0 // assert: DumpStructure.structure
  given Int = 1
  given x: String = ""
  given [A]: List[A] = Nil
  given (using i: Int): Long = 1L
