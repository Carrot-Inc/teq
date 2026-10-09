// expect: 22:26: error: type mismatch: found String, required Int
// expect: 1 error found
// `h`'s body is typed on demand inside the first alternative of `pair`, after that alternative's
// own mismatch on `2`; the alternative is retracted keeping the body's error, which stays the
// definition's (its promoted range relocated with it, not clamped first), so the outer `combine`'s
// first alternative is not given up for it, and the error is reported, as scalac reports it.
trait Ops:
  extension [T](x: Int)
    def combine(y: String, z: String): String = "s " + y + z
  extension [T](x: Int)
    def combine(y: Any, z: String): String = "a " + y + z

trait Ops2:
  extension [T](x: Int)
    def pair(y: String, z: String): String = "s" + y + z
  extension [T](x: Int)
    def pair(y: Int, z: String): String = "i" + y + z

object Main extends Ops, Ops2:
  def main(args: Array[String]): Unit =
    println(this.combine[Unit](1)(this.pair[Unit](1)(2, h), "z"))
  def h = { val q: Int = "x"; "b" }
