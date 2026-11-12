// scalac 3.8.4 runs no phase after a typer that reported an error (`Phase.isRunnable`), so PostTyper's bound checks
// (`Checking.checkAppliedType`'s `checkBounds`, E057) say nothing in a program with a typer error. Here the member's
// F-bounded lower bound `BS[?, S]` is E140, scalac's one
// error (8:15). The check of `H.this.S` against `BS`'s bound
// `S <: BS[T, S]`, at 8:26, is PostTyper's: no error is expected there.
// scalac 3.8.4: rejected, 1 error(s).
trait BS[T, S <: BS[T, S]]
trait H { type S >: BS[?, S] }
// expect: 8:16: error: illegal cyclic type reference: lower bound BS[?, H.this.S] of type S refers back to the type itself
// expect: 1 error found
// absent: 8:27:
