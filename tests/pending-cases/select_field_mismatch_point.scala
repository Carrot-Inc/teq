// A mismatch is reported at its tree's point, a `Select`'s at its name, the whole selection underlined (`errorTree`,
// `srcPos`): scalac 3.8.4 reports `val s: String = O.r` and `h.r` at `r`. teq's typed selection of a field keeps its
// prefix's span (`O`, `h`), so the report's point is the selection's start, its underline whole; master (efddb38de3)
// and the head (61c1af7941) alike: queued, typedSelect's span (A5's, cut-a5-select-addendum). Its found type is the
// path's, `(O.r : R)`, where teq shows `R`. The qualifiers of view_mismatch_select_qualifier_position keep the same
// span.
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
