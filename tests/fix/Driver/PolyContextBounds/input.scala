/*
rules = ["file:../../support/Params.scala"]
*/
// A polymorphic function's binder takes context bounds, its `Type.Param`'s children (scalameta's `cbounds`), and a
// type lambda's may be `_` (dotty's `typeParam` by owner; the review of `340567fdd`). The evidences the bounds stand
// for are no node.
package bounds

trait Ord[X] // assert: Params.param
type W = [_] =>> Int // assert: Params.param
val less = [X: Ord] => (x: X, y: X) => summon[Ord[X]] != null // assert: Params.param
type Less = [Y: Ord as ord] => (Y, Y) => Boolean // assert: Params.param
