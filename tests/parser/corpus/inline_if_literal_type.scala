// `inline if` reduces on a condition of a literal type, as `valueOf` of a literal type, an
// alias of one and a class's type member give it.
trait Flags:
  final type debug = false
  inline def log(s: String): Unit = inline if valueOf[Flags#debug] then println(s) else println("quiet")
  inline def log2(s: String): Unit = inline if valueOf[true] then println(s) else ()
object F extends Flags
object Main:
  def main(args: Array[String]): Unit =
    F.log("x")
    F.log2("y")
