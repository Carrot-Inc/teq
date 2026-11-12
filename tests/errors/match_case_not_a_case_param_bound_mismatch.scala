// expect: 8:33: error: this match type case is not a match case once its pattern is approximated: Nothing
// expect: 1 error found
// scalac 3.8.4 rejects this program with a type mismatch (E007: Found (c : C[?, ?]), Required C[?, ? <: Inv[Int]]),
// not a crash: comparing the parameter bounds builds the match type over the case its approximation leaves no
// MatchCase application (`Nothing`), which that comparison does not reduce. teq is to report that case as no match
// case where scalac reports the mismatch.
class Inv[X]
class C[A, B <: Inv[Int match { case A => String }]]
def h(c: C[?, ?]): C[?, ? <: Inv[Int]] = c
@main def main(): Unit = println("ok")
