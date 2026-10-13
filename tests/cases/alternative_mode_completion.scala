// A pattern alternative's mode belongs to its pattern's context (`Typer.typedAlternative`): the
// `unapply` whose result `case E(_) | E(_)` makes the typer infer is typed in its own context, its
// body's binder `case z => z` no binder of an alternative.
def f(x: Any): Any = x match
  case E(_) | E(_) => 1
  case _ => 0
object E:
  def unapply(x: Any) =
    val y = x match { case z => z }
    if y == 3 then Some(y) else None

@main def run(): Unit =
  println(f(3))
  println(f(4))
