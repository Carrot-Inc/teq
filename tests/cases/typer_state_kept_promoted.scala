// `h`'s body is typed on demand inside the first
// alternative of `pair` and warns; scalac applies the outer
// `combine`'s first alternative: `s i2bz` with the warning.
trait Ops:
  extension [T](x: Int)
    def combine(y: String, z: String): String = "s " + y + z
  extension [T](x: Int)
    def combine(y: String, z: Int): String = "i " + y + z

trait Ops2:
  extension [T](x: Int)
    def pair(y: String, z: String): String = "s" + y + z
  extension [T](x: Int)
    def pair(y: Int, z: String): String = "i" + y + z

object Main extends Ops, Ops2:
  def main(args: Array[String]): Unit =
    println(this.combine[Unit](1)(this.pair[Unit](1)(2, h), "z"))
  def h = { 1; "b" }
