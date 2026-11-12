// Pinned for the queued part inlining-lila-folds.
// scalac accepts and prints the expected output. An
// `if` over an `Int` constant and a nested `if` of
// `Float` constants, against the open result variable
// of `fold`: the constant becomes a `Float` (dotty's
// `harmonize`, reading the nested branches' union
// as its members' class) and the constraint the
// branches added before harmonisation is restored
// (`Applications.harmonic`), so the result is a `Float`.
def isObvious(t: String) = t.length > 3
def weightOf(angle: Option[String], win: Boolean) =
  angle.fold(1f): theme =>
    if theme == "mix" then 1
    else if isObvious(theme) then if win then 0.1f else 0.4f
    else if win then 0.7f
    else 0.8f
extension (f: Float) def atLeast(m: Float): Float = math.max(f, m)
def player(angle: Option[String], win: Boolean, p: Float): Int =
  ((weightOf(angle, win) + p).atLeast(0.1f) * 10).toInt
object Main:
  def main(args: Array[String]): Unit =
    println(player(Some("mix"), true, 0.5f))
    println(player(Some("obvious"), false, -0.2f))
    println(player(Some("ab"), true, 0f))
    println(player(None, true, 0f))
