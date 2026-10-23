/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Defn.ExtensionGroup, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Defn_ExtensionGroup

object Defn_ExtensionGroup:
  val marker = 0 // assert: DumpStructure.structure
  extension (x: Int) def twice = x + x
  extension (s: String)
    def once = s
  extension (b: Boolean)
    def no = !b
    def yes = b
  extension [A](a: A)(using Ordering[A]) def same = a
