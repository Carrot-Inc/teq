// A method's explicit type argument is checked by PostTyper's `checkBounds` (`Checking.scala` 72-80): a simple kind for a
// formal whose bound is no type lambda, the bound's subtyping otherwise. An application of a higher-kinded member
// (`P#F[Int]`, of the kind `[Y] =>> Any`) is below an `F[_]` formal's bound `[_] =>> Any`, which scalac accepts.
trait P { type F[X] <: [Y] =>> Any }
def f[F[_]]: Int = 1

@main def run(): Unit = println(f[P#F[Int]])
