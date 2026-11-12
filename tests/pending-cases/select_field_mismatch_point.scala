// A mismatch is reported at its tree's point, a `Select`'s
// at its name, the whole selection underlined (`errorTree`,
// `srcPos`): scalac 3.8.4 reports `val s: String = O.r`
// and `h.r` at `r`. Its found type is the path's,
// `(O.r : R)`. teq is to report them so. Queued:
// typedSelect's span (A5's, cut-a5-select-addendum).
class R
object O:
  val r: R = new R
class H:
  val r: R = new R
object Main:
  def main(args: Array[String]): Unit =
    val h = new H
    val s: String = O.r
    val t: String = h.r
