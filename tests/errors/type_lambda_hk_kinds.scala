// expect: 14:10: error: type argument Option does not have the same kind as its bound [G[_]]
// expect: 15:10: error: type argument Wrap does not have the same kind as its bound [G[_]]
// expect: 16:10: error: type argument Option does not have the same kind as its bound [_,_]
// expect: 17:13: error: type argument Hold does not have the same kind as its bound [_]
// expect: 4 errors found

// An applied type's argument has its parameter's kind, the kinds of the parameters each takes
// compared recursively from the bounds the binders keep (`Checking.preCheckKind`,
// `TypeApplications.hasSameKindAs`); `Nothing` is of every kind.
class Wrap[G[H[_]]](val x: Int)
class Hold[G[_]](val g: Int)
type T = [F[G[_]]] =>> Int
type B[F[_, _]] = Int
val a: T[Option] = 1
val d: T[Wrap] = 2
val e: B[Option] = 3
val i: Hold[Hold] = ???
val f: T[Hold] = 4
val h: T[Nothing] = 5
val j: Hold[[X] =>> Option[X]] = ???
val k: T[[G[_]] =>> Int] = 6
@main def run(): Unit = println(f + h + k)
