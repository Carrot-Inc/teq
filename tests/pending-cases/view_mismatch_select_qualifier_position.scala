// The view search's ambiguity is the qualifier's mismatch, reported at the qualifier's point, a `Select`'s at its name,
// the whole qualifier underlined (`errorTree`, `srcPos`; Typer.scala:4346-4353): scalac 3.8.4 reports `O.d` at `d`,
// `O.P.q` at `q`, `h.r` at `r`. teq reports its own ambiguity at the selection's start, master (efddb38de3) and the
// head (61c1af7941) alike; the head names the receivers `(O.P.q : R)` and `(h.r : R)` as scalac does, where master says
// `R`. The last line's `pick2`, no member, is reported at its name on the head, scalac's point, and at the selection's
// start on master. Queued: push 6 (typeMismatch whole, at the qualifier, whose typed tree of a field keeps its prefix's
// span: select_field_mismatch_point).
class R
trait Ops:
  extension (r: R) def pick: String
object R:
  given a: Ops with
    extension (r: R) def pick: String = "a"
  given b: Ops with
    extension (r: R) def pick: String = "b"
object O:
  val r: R = new R
  def d: R = new R
  object P:
    val q: R = new R
class H:
  val r: R = new R
object Main:
  def main(args: Array[String]): Unit =
    val h = new H
    println(O.d.pick)
    println(O.P.q.pick)
    println(h.r.pick)
    println(O.r.pick2)
