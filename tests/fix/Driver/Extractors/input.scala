/*
rules = ["file:../../support/Extractors.scala"]
*/
// The extractors as scalameta 4.17.4's (the review of e9bffbf79): `Defn.Var`'s legacy `rhs`, Some and None, and the
// formats and value `Lit.Double`, `Lit.Float` and `Lit.String` give, each a `String`.
class Extractors {
  var initialised = 1 // assert: Extractors.initialised
  var deferred: Int = _ // assert: Extractors.deferred
  val d = 1.25 // assert: Extractors.double
  val f = 1.25f // assert: Extractors.float
  val s = "hello" // assert: Extractors.string
}
