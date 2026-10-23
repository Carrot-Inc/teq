/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Stat.Block, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Stat_Block

object Stat_Block:
  val marker = 0 // assert: DumpStructure.structure
  trait A
  type R = A { type T; def m: Int }
  type S = { val v: Int }
