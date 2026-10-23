/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Name.Anonymous: an anonymous parameter, constructor, init and given, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Name_Anonymous

object Name_Anonymous:
  val marker = 0 // assert: DumpStructure.structure
  def f(using Int) = 1
  class C
  val n = new C
  given Int = 1
