/*
rules = ["file:../../support/Params.scala"]
*/
// A type lambda's and a polymorphic function's binders are `Type.Param`s of their trees, as dotty's `lambdaAbstract`
// keeps its `TypeDef`s (the review of `86884ea36`).
package binders

type T = [A] =>> List[A] // assert: Params.param
type P = [B <: AnyRef] => B => B // assert: Params.param
val f = [C] => (c: C) => c // assert: Params.param
