package q03
object Locals:
  def m: Int =
    case class LC(a: Int)
    val lc = LC(1)
    class K { val (u, v) = (lc.a, List(1)) ; val Some(w) = Option(1): @unchecked ; def this(z: String) = this() }
    lc.a
  def fwdImplicit: Int =
    import scala.language.implicitConversions
    def n: Int = "abc"
    implicit def conv(s: String): Int = s.length
    n
  def lp: Any =
    val p = [T] => (t: T) => t
    val d = (x: Int) => List(x)
    p
  def ann: Int =
    @deprecated("x", "1") val q = 1
    @inline def r = q
    r
