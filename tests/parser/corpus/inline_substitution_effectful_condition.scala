// teq: --inline-substitution
// An ordinary `if` whose condition has a constant type selects its branch, the condition kept
// before it where it is no idempotent expression (scalac 3.8.4's `InlineTyper.typedIf`): `f` prints
// `condition` then 1, and `error("dead")` is never expanded. The retype path folds only a
// condition that is a constant tree and reports `dead` here.
import scala.compiletime.error
def cond(): true = { println("condition"); true }
inline def f: Int = if cond() then 1 else error("dead")
@main def run(): Unit = println(f)
