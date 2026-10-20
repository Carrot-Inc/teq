// expect: Type argument [X <: A] =>> List[X] does not conform to upper bound [_] =>> Any
// scalac 3.8.4: -- [E057] Type Mismatch Error: Test.scala:2:25 ... Type argument [X <: A] =>> List[X] does not conform to upper bound [_] =>> Any
// (`ProtoTypes.wildApprox` maps a type lambda's parameter bounds with the lambda's own parameters kept, so the
// default's expected type keeps the bound `X <: A` that the kind check rejects; teq's default-prototype map drops
// it and accepts). The review of 5010f41f8, finding 5; the queued part inlining-lila-folds.
class Box[F[_]]
def f[A](x: Box[[X <: A] =>> List[X]] = new Box[[X] =>> List[X]]): Any = x
