/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// The pass's UnaryString probe: a unary operator on a string literal, a `Lit.WithUnary` as scalameta 4.17.4's
// `rawLiteral` makes it (structure.txt).
package nodes.Lit_WithUnary_String

object O:
  extension (s: String) def unary_! : Int = s.length
  val n = !"hello"
  val m = !"""multi"""

object Marker:
  val marker = 0 // assert: DumpStructure.structure
