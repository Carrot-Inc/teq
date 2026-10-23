/*
rules = ["file:../../support/DumpStructure.scala"]
*/
// Template, its fields as scalameta 4.17.4's (structure.txt).
package nodes.Template

object Template:
  val marker = 0 // assert: DumpStructure.structure
  class B
  trait T
  class C extends B with T:
    val x = 1
  case class D(x: Int) derives CanEqual
  object E extends B
