// expect: 11:12: error: type argument Int does not have the same kind as its bound [_]
// expect: 1 error found
// A class application whose argument the kind check refuses (`preCheckKind`, Typer.scala 2794-2802): dotty's
// `errorTree` makes the argument the error type before `assignType`, and PostTyper's bound check of the other argument
// instantiates `A <: F[Int]` with it, `<error>[Int]`, which every type is below: scalac 3.8.4 reports the kind error
// alone (1 error),
// and so does teq: no
// "type argument String does not conform to upper bound Int[Int]"
// beside the kind error.
class C[F[_], A <: F[Int]]
type X = C[Int, String]
