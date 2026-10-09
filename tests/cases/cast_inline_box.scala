// An inline method's argument of a bottom type, `null`'s `Null`, binds at the parameter's type
// (`Inliner.paramBindingDef`, `isBottomTypeAfterErasure`), so a cast of the parameter to
// `BoxedUnit` sees an `Any` and gives `()`, by value (a literal standing for itself, a call bound
// to a val), `inline` or by name; a `Null`-typed operand written as such keeps its `null`.
inline def box(x: Any): scala.runtime.BoxedUnit = x.asInstanceOf[scala.runtime.BoxedUnit]
inline def boxInline(inline x: Any): scala.runtime.BoxedUnit = x.asInstanceOf[scala.runtime.BoxedUnit]
inline def boxByName(x: => Any): scala.runtime.BoxedUnit = x.asInstanceOf[scala.runtime.BoxedUnit]
inline def boxNull(x: Null): scala.runtime.BoxedUnit = x.asInstanceOf[scala.runtime.BoxedUnit]
var calls = 0
def nothingThere: Null = { calls += 1; null }
@main def run(): Unit =
  println(box(null) == null)
  println(box(nothingThere) == null)
  println(calls)
  println(boxInline(null) == null)
  println(boxByName(null) == null)
  println(boxNull(null) == null)
  println(box("text") == null)
