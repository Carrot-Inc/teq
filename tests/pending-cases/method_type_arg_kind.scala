// A method's explicit type argument is checked by PostTyper's `checkBounds` (`Checking.scala`): a simple kind for a
// formal of no kind of its own, the bound's subtyping otherwise. teq checks it by arity (`type_arity`), master's, so an
// application of a higher-kinded member (`P#F[Int]`, of the kind `[Y] =>> Any`) is refused for an `F[_]` formal that
// scalac accepts: a part of its own (found by the fold of the review of `bb04d0aaf`; a constructor's arguments,
// an applied type tree's, are `preCheckKind`'s).
trait P { type F[X] <: [Y] =>> Any }
def f[F[_]]: Int = 1

@main def run(): Unit = println(f[P#F[Int]])
