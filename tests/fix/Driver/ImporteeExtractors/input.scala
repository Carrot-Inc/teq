/*
rules = ["file:../../support/Importees.scala"]
*/
// The given importees' extractors, scalameta's `Importee.Given(tpe)` and `Importee.GivenAll()`, beside `Wildcard()`.
package driver.importees
object O:
  given Int = 1
  given String = ""
import O.{given Int} // assert: Importees.given
import O.given // assert: Importees.givenAll
import O.* // assert: Importees.wildcard
