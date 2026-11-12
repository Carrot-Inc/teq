// An application of a curried alias (`Const[Long][Int]` of
// `type Const[A] = [X] =>> A`) is `Long` (`AppliedType.dealias` through
// the constructor's alias), so `typedNumber`'s `isRef(LongClass)` makes
// `1` a `1L`, `classBound` converts a constant to `Byte`, and the alias's
// members are `Long`'s, as scalac 3.8.4 accepts it. A part of its own.
type Const[A] = [X] =>> A
def id[T](x: T): T = x
val x: Const[Long][Int] = 1
val b: Const[Byte][Int] = 65
val c: Const[Byte | Null][Int] = id[66](66)

@main def run(): Unit = println(s"${x + 1L} $b $c")
