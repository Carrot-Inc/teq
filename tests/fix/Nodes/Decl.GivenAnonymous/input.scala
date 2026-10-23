/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Decl.GivenAnonymous, its fields as scalameta 4.17.4's (structure.txt): a given of no body, which scalac 3.8.4 makes
// an instance of its type.
package nodes.Decl_GivenAnonymous

trait Show
trait Eq[A]

object Decl_GivenAnonymous:
  val marker = 0 // assert: DumpStructure.structure
  given Show
  given Eq[Int]
