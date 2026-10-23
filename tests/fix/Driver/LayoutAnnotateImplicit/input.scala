/*
rules = ["file:../../support/DumpLayout.scala"]
*/
// Each written annotation ascription a `Term.Annotate` of its own, its annotations its own (nested in parentheses, two
// in one ascription, an ascription of `@nowarn` alone inside and outside parentheses), and an implicit lambda's
// parameter and clause from its `implicit`, a `Mod.Implicit` each (the review of e0d4a9c4f), as scalameta 4.17.4's.
import scala.annotation.nowarn
object AnnotateImplicit:
  val marker = 0 // assert: DumpLayout.layout
  val nested = ((1): @unchecked): @unchecked
  val twoInOne = (1: @unchecked @nowarn)
  val nowarnInside = (((1): @nowarn): @unchecked): @unchecked
  val nowarnOutside = ((1: @unchecked)): @nowarn
  val blockImplicit: Int => Int = { implicit x => x }
  val exprImplicit: Int => Int = implicit y => y
