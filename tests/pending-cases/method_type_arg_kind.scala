// A method's explicit type argument is checked by PostTyper's `checkBounds`
// (`Checking.scala`): a simple kind for a formal of no kind of its own, the bound's
// subtyping otherwise. scalac accepts an application of a higher-kinded member
// (`P#F[Int]`, of the kind `[Y] =>> Any`) for an `F[_]` formal: a part of its
// own (a constructor's arguments, an applied type tree's, are `preCheckKind`'s).
trait P { type F[X] <: [Y] =>> Any }
def f[F[_]]: Int = 1

@main def run(): Unit = println(f[P#F[Int]])
