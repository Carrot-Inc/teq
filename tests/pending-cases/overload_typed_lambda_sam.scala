// An overloaded application's function literal with written
// parameter types is typed once without an expected type
// (`FunProto.typedArg` caches `typedUnadapted`), and adapted
// to each alternative's formal (`adapt`: `Int => Int`
// directly, the SAM trait `F` by conversion), so both
// alternatives are applicable and the second argument's
// type picks the function alternative: scalac prints
// FUN. Pinned for the queued part inlining-lila-folds.
trait F { def run(x: Int): Int }
def f(x: F, n: Int): String = "SAM"
def f(x: Int => Int, n: String): String = "FUN"
@main def main(): Unit = println(f((x: Int) => x + 1, "s"))
