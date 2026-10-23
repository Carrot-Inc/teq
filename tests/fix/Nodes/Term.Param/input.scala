/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Term.Param, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Term_Param

object Term_Param:
  val marker = 0 // assert: DumpStructure.structure
  def f(x: Int = 1, y: => Int, z: Int*) = x
  class C(val a: Int, var b: Int, private val c: Int = 2)
  inline def g(inline x: Int) = x
  def h(implicit i: Int) = i
