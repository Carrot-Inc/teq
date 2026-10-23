/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Type.Singleton, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Type_Singleton

object Type_Singleton:
  val marker = 0 // assert: DumpStructure.structure
  val x = 1
  type S = x.type
  class C:
    def f: this.type = this
