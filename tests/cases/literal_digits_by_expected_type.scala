// An integer literal's digits read by the type expected, as scalac's `typedNumber` reads them: a `Long`'s by
// `longFromDigits`, a `Float`'s or a `Double`'s (but for hexadecimal digits) as a floating number, any other's as an
// `Int`'s, then converted. A literal against a type variable is an `Int`'s unless the variable is pinned to a class
// (an invariant `C[T]` expected as `C[Long]`); an overloaded operator's or method's argument is an `Int`'s. Doubles are
// compared, not printed (JavaScript writes them its own way).
class C[T](val x: T)
object Main:
  def h[T](x: T): T = x
  def ov(x: Int): String = "I" + x
  def ov(x: Long): String = "L" + x
  def main(args: Array[String]): Unit =
    val a: Long = 0xffffffff
    val b: Long = 0b11111111111111111111111111111111
    val c: Int = 0b1010
    val d: Double = 0b1010
    val e: Double = 0xffffffff
    val g: Long = -0xffffffff
    val i: Int = -0xffffffff
    val j: Double = 123456789012345678901234567890
    println(s"$a $b $c ${d == 1010.0} ${e == -1.0} $g $i ${j == 1.2345678901234568e29}")
    val t: (Long, Int) = (0xffffffff, 1)
    val nt: (n: Long, m: Int) = (n = 0xffffffff, m = 1)
    val l: List[Long] = List(0xffffffff)
    val o: Option[Long] = Some(0xffffffff)
    val y: Long = h(0xffffffff)
    val z: Long = identity(0xffffffff)
    println(s"${t._1} ${nt.n} ${l.head} ${o.get} $y $z")
    val k: C[Long] = new C(0xffffffff)
    val n: Long = 4294967295L
    println(s"${k.x} ${n + 0xffffffff} ${ov(0xffffffff)}")
    val p = n match
      case 0xffffffff => "matched"
      case _ => "not"
    val q = n match
      case m if m >= 0x80000000 && m <= 0xffffffff => 1
      case _ => 0
    println(s"$p $q ${1_000} ${0x_ff} ${010}")
