/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// The pass's DecimalTrivia probe: a sign folded into a decimal across a comment, its value scalameta 4.17.4's reader's
// (structure.txt), the comment in its syntax and none of its digits.
package nodes.Lit_Double_SignTrivia

object O {
  val x = - /* sign */ 1.25
  val y = -  2.50f
  val z = - /* zero */ 0.0
}

object Marker:
  val marker = 0 // assert: DumpStructure.structure
