/*
rules = ["file:../../support/Params.scala"]
*/
// dotty's tests/pos/i13541.scala at 3.8.4 (the review of `86884ea36`): `Foo`'s `B` and its lambda's `A` are both
// `Type.Param`s, and so are the context-bounded `A`s of `fooUnit` and `f`.
trait F[A] // assert: Params.param
trait Z
object Z:
  given F[Z] = ???

type Foo[B] = [A] =>> Bar[A, B] // assert: Params.param
trait Bar[A, B] // assert: Params.param

given fooUnit[A: F]: Foo[Unit][A] = ??? // assert: Params.param
//given bar[A: F]: Bar[A, Unit] = ???

def f[A: F](using Foo[Unit][A]): Nothing = ??? // assert: Params.param

def broken: Nothing = f[Z]