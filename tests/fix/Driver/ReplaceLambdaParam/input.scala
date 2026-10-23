/*
rules = ["file:../../support/ReplaceLambdaParam.scala"]
*/
// A lambda parameter whose type is in parentheses spans them: replaced whole, an ordinary lambda's and a colon
// argument's, nothing of the type left (the pass over 2865c7af2; its output scalafix-core 0.14.9's).
package driver.replacelambdaparam
object Subject:
  val ordinary = (x: (Int)) => x
  val colon = List(1).map: (x: (Int)) =>
    x
