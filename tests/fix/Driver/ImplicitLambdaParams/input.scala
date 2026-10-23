/*
rules = ["file:../../support/ImplicitParams.scala"]
*/
// An implicit lambda's parameter and clause from its `implicit`, a `Mod.Implicit` each, as scalameta's
// `implicitClosure` (the review of e0d4a9c4f); a method's implicit clause beside it.
object ImplicitLambdaParams {
  val block: Int => Int = { implicit x => x } // assert: ImplicitParams.clause
  val expr: Int => Int = implicit y => y // assert: ImplicitParams.clause
  def method(implicit z: Int): Int = z // assert: ImplicitParams.clause
}
