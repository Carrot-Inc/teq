/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.Repeated, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_Repeated

object Term_Repeated:
  val marker = 0 // assert: DumpStructure.structure
  def f(xs: Int*) = xs.sum
  val a = f(List(1)*)
  val b = f(List(1): _*)
