// expect: 11:12: error: type argument Int does not have the same kind as its bound [_]
// expect: 1 error found
// A class application whose argument the kind check refuses (`preCheckKind`, Typer.scala 2794-2802): dotty's
// `errorTree` makes the argument the error type before `assignType`, and PostTyper's bound check of the other argument
// instantiates `A <: F[Int]` with it, `<error>[Int]`, which every type is below: scalac 3.8.4 reports the kind error
// alone (1 error), and so does teq since 50858215f6 (`check_type_args_through` runs the kind check over every
// argument first and checks the bounds with a refused one the error type). From lane A to 24df636e49 teq added "type
// argument String does not conform to upper bound Int[Int]" (`?` before c72973e59e); the round's base a5f69ed337
// reported the one error. The closure adversary of the pass on 5bd85bcd1 (round 23).
class C[F[_], A <: F[Int]]
type X = C[Int, String]
