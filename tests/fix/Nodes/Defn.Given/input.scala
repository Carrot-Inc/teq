/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Defn.Given, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Defn_Given

object Defn_Given:
  val marker = 0 // assert: DumpStructure.structure
  trait Show[A]:
    def show(a: A): String
  given Show[Int] with
    def show(a: Int) = a.toString
  given named: Show[String] with
    def show(a: String) = a
  given [A]: Show[List[A]] with
    def show(a: List[A]) = ""
