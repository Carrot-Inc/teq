/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Pkg: a package clause and a package block, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Pkg
package inner:
  val y = 1

object Pkg:
  val marker = 0 // assert: DumpStructure.structure
