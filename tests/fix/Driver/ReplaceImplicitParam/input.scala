/*
rules = ["file:../../support/ReplaceImplicitParam.scala"]
*/
// An implicit lambda's parameter spans its `implicit`: replaced with its name, the keyword gone with it (the review of
// e0d4a9c4f; the output scalafix-core 0.14.9's).
package driver.replaceimplicitparam
object Subject {
  val block: Int => Int = { implicit x => x }
  val expr: Int => Int = implicit y => y
}
