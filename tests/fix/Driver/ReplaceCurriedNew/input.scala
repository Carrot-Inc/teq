/*
rules = ["file:../../support/ReplaceNew.scala"]
*/
// A curried constructor call is one `Term.New`, its `Init` over both clauses: replaced whole, nothing of it left
// (the pass's CurriedNew probe; scalafix writes the same).
package driver.replacecurriednew
class Box(x: Int)(y: Int)
object Subject { val value: Box = new Box(1)(2) }
