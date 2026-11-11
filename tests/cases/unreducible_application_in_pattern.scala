// teq: --wconf msg=cannot.be.checked.at.runtime:s
// `Checking.checkWildcardApply` (Checking.scala 131-141, E043) in a pattern: dotty types a wildcard argument of a
// typed pattern or of a match type's case pattern as a pattern-bound type symbol (`F[_]`; the closure adversary's
// probe SRC-E043-pattern reads a `TypeRef` flagged `case`; `typedArg`'s `typedType(.., mapPatternBounds = true)`), no
// `TypeBounds`, so the re-application under `Mode.AllowLambdaWildcardApply` has no wildcard argument and reports
// nothing: scalac 3.8.4 accepts the program, with E092's unchecked warnings at the two typed patterns, and so does teq
// since 2ca0369a50 (no E043 in a pattern the typer types,
// `pattern_typed`). 0ca2e5d47c reported E043 at all three; the
// round's base a5f69ed337 accepted it. The closure adversary of the pass on 5bd85bcd1 (round 23).
class H:
  type KC[-X]
val h = new H
type App[F[_], A] = F[A]
def f[F[_]](x: Any): Int = x match
  case y: F[?] => 1
  case _ => 0
def g(x: Any): Int = x match
  case y: App[[X] =>> h.KC[X], ? <: Int] => 1
  case _ => 0
type M[F[_], X] = X match
  case F[?] => Int
  case _ => String
@main def run(): Unit =
  val m: M[List, List[Int]] = 1
  println(s"${f[List](1)} ${g(1)} $m")
