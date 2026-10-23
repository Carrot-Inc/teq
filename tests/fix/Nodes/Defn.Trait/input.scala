/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Defn.Trait, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Defn_Trait

object Defn_Trait:
  val marker = 0 // assert: DumpStructure.structure
  trait A
  trait B[T](x: T) extends A
  sealed trait C
