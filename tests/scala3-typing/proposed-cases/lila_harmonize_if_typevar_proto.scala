// lila: puzzle/PuzzleFinisher.scala:210/214 (`weightOf` = `angle.asTheme.fold(1f): theme => if ... then 1 else ... 0.1f`)
// When the expected type is not fully defined (here fold's B), the numeric branches of an if
// are harmonized to one numeric type (Typer.typedIf via harmonic/harmonize): 1 becomes 1.0f.
def weightOf(theme: Option[Int]) =
  theme.fold(1f): t =>
    if t == 0 then 1
    else if t == 1 then 0.1f
    else 0.8f

@main def run() =
  val w: Float = weightOf(Some(0))
  println(w)
  println(weightOf(Some(1)) + weightOf(None))
