// A pattern-bound type variable at a higher-kinded parameter (`case C[f]` of a match type, `case c: C[f]` of a typed
// pattern, `C[F[_]]`): dotty gives the variable the parameter's kind (`f[Int]` is `List[Int]` for `C[List]`) and scalac
// 3.8.4 accepts the program. teq reports "type argument f does not have the same kind as its bound [_]" at both, and
// since 24df636e49 makes the argument the error type, the match type no longer reduces (a second error at `m`). The
// closure adversary of the pass on 5bd85bcd1 (round 23); the round's base reported the two kind errors.
class C[F[_]]
type M[X] = X match
  case C[f] => f[Int]
  case _ => String
val m: M[C[List]] = List(1)
def g(x: Any): Int = x match
  case c: C[f] => 1
  case _ => 0
@main def run(): Unit = println(s"$m ${g(new C[List])}")
