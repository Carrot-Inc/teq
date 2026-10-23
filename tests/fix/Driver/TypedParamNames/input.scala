/*
rules = ["file:../../support/TypedNames.scala"]
*/
// A typed parameter's and a named tuple element's name is its value, its backquotes off (the pass's QuotedName
// probe).
package driver.typedparamnames
object Subject:
  type F = (`a b`: Int) => String // assert: TypedNames.name
  type T = (
    `c d`: Int, // assert: TypedNames.name
    e: String // assert: TypedNames.name
  )
