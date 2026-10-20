// lila: challenge/ChallengeGranter.scala:86, study/Chapter.scala:86, puzzle/PuzzleOpening.scala:90
// Without -strictEquality, `Option[X] == X` is allowed when neither side has a reflexive
// CanEqual (Synthesizer.synthesizedCanEqual, validEqAnyArgs/hasEq); Option[X] has one only if X does.
case class Denorm(fen: String)
object Keys:
  opaque type PerfKey = String
  object PerfKey:
    val ultraBullet: PerfKey = "ultraBullet"
import Keys.PerfKey

@main def run() =
  val perfKey: Option[PerfKey] = Some(PerfKey.ultraBullet)
  println(perfKey == PerfKey.ultraBullet)
  val denorm: Option[Denorm] = Some(Denorm("x"))
  println(denorm.filter(denorm != _))
