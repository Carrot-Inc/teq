/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// The pass's WithType probe, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_With_Self

trait A
trait B
trait Subject { self: A with B => }

object Marker:
  val marker = 0 // assert: DumpStructure.structure
