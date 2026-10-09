// teq: --inline-substitution
// An ordinary `if` whose condition an `inline` argument makes a constant selects its branch where
// the method expands, the other one never expanded: `f(true)` is 1 and `error("dead")` stays
// unexpanded, in scalac 3.8.4 (`InlineTyper.typedIf`) as in the expansion by substitution.
import scala.compiletime.error
inline def f(inline b: Boolean): Int =
  if b then 1 else error("dead")
@main def run(): Unit = println(f(true))
