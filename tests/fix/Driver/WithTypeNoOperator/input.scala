/*
rules = ["file:../../support/Operators.scala"]
*/
// `A with B` is a `Type.With`, no infix type with an operator to read; `A & B` is one (the pass's WithType probe).
package driver.withtypenooperator
trait A
trait B
trait Subject { self: A with B => }
type Both = A & B // assert: Operators.op
