// teq: --threads 1
// expect: 23:21: error: type mismatch
// expect: 1 error found
// At one worker `H.ext`'s body is typed on demand inside the first extension alternative and its
// warning is its definition's (a promoted range); the extractor's quiet typing of the path drops
// what it reported, and the range goes with it (`state::diags_cut_at`), so the alternative's own
// mismatch on `"z"` is its failure and the second alternative applies, as in scalac and master.
// The one error is the last line's.
trait Ops:
  extension [T](x: Int)
    def combine(y: String, z: Int): String = "s " + y
  extension [T](x: Int)
    def combine(y: String, z: String): String = "i " + y
object O extends Ops

object Ext:
  def unapply(s: String): Option[String] = Some(s)

@main def run(): Unit =
  println(O.combine[Unit](1)("a" match { case H.ext(y) => y }, "z"))
  val n: Int = 1
  println(n)
  val bad: String = n

object H:
  val ext = { 1; Ext }
