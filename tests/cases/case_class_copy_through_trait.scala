// The copy of a case class is a member of a trait extending it, of what the trait's type stands for:
// scalachess's `val good: MoveAssessment = new Glyph(1, "!", "Good move") with MoveAssessment`, which
// lila copies by name.
case class Glyph(id: Int, symbol: String, name: String)

object Glyph:
  sealed trait MoveAssessment extends Glyph
  object MoveAssessment:
    val good: MoveAssessment = new Glyph(1, "!", "Good move") with MoveAssessment
    val mistake: MoveAssessment = new Glyph(2, "?", "Mistake") with MoveAssessment

@main def run(): Unit =
  import Glyph.MoveAssessment.*
  val renamed: Glyph = good.copy(name = "Bien")
  println(renamed)
  println(List(good, mistake).map(_.copy(symbol = "*")))
