// Pinned for the queued part inlining-lila-folds: the commit that made this case pass (db0a8ede5, the lila folds'
// eta-expansion and context-function changes) broke tests/scala3's boehm-berarducci (scala3-run and scala3-interp)
// and pos/i12211, pos/i3945 against master (the gate of 5010f41f8), a method value eta-expanded against an expected
// function type with its variables solved to Any; reverted on 2026-10-20. scalac accepts and prints the expected
// output.
// An alias of a union of literals stays the type argument of a conversion applied to a
// receiver of that type: `ph -> 1` for a `ph: GamePhase` is `ArrowAssoc[GamePhase]`'s, as
// dotty's view takes the qualifier's type widened without dealiasing (`inferView`'s
// `ViewProto(argument.tpe.widen, ..)`); a member of the union is still its join's.
type GamePhase = "opening" | "middlegame" | "endgame"
val phaseNames: List[GamePhase] = List("opening", "middlegame", "endgame")
def pairs(infos: List[Int]): List[(GamePhase, Int)] =
  for
    phase <- phaseNames
    slice = infos.filter(_ > 0)
    n <- slice.headOption
  yield phase -> n
def one(ph: GamePhase): (GamePhase, Int) = ph -> ph.length
object Main:
  def main(args: Array[String]): Unit =
    println(pairs(List(1, 2)))
    println(one("endgame"))
