// An overloaded application's function literal with written parameter types is typed once without an expected
// type (`FunProto.typedArg` caches `typedUnadapted`), and adapted to each alternative's formal (`adapt`:
// `Int => Int` directly, the SAM trait `F` by conversion), so both alternatives are applicable and the second
// argument's type picks the function alternative: scalac prints FUN. teq types the literal against the first
// alternative's formal and keeps that typing for every alternative (`pretype_typed_lambda`), so the literal
// adapted to `F` matches no alternative with the `String` argument (error: None of the overloaded alternatives
// of method f ... match arguments (F, "s")). The review of 5010f41f8, finding 1; the queued part
// inlining-lila-folds.
trait F { def run(x: Int): Int }
def f(x: F, n: Int): String = "SAM"
def f(x: Int => Int, n: String): String = "FUN"
@main def main(): Unit = println(f((x: Int) => x + 1, "s"))
