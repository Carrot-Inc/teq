// The trigger of `interp-char-is-digit` in proptests/src/known.rs, shrunk by tests/prop-known.sh:
// 2 of the program's lines differ, of 1 cases
// line 0.f.v: the targets print
//     interpreter: -1
//     javascript: -256
//     scalac: -256
// folded: (~(((Consts.kc0 * Consts.kc1).toChar match { case '0' => Consts.kc2; case '\u0000' => Consts.kc3; case n if n.isDigit => Consts.kc4; case _ => Consts.kc5 })))
// at run time: (~(((p0 * p1).toChar match { case '0' => p2; case '\u0000' => p3; case n if n.isDigit => p4; case _ => p5 })))
//   where inline val kc0 = 63L
//   where inline val kc1 = 3
//   where inline val kc2 = 2147483647
//   where final val kc3 = -1
//   where inline val kc4 = 0
object Consts:
  final val none = 0
  val written: 1 = 1
  inline val k0c0 = 63L
  inline val k0c1 = 3
  inline val k0c2 = 2147483647
  final val k0c3 = -1
  inline val k0c4 = 0
  inline val k0c5 = 255

object Vars:
  var none: Int = 0


def units(text: String): String =
  var out = ""
  var i = 0
  while i < text.length do
    out = out + text.charAt(i).toInt + " "
    i += 1
  out

def show(id: String, text: => String): Unit =
  val shown =
    try text
    catch
      case e: ArithmeticException => "ArithmeticException"
      case e: IndexOutOfBoundsException => "IndexOutOfBoundsException"
      case e: NumberFormatException => "NumberFormatException"
  println(id + ":" + shown)

def r0v(p0: Long, p1: Int, p2: Int, p3: Int, p4: Int, p5: Int): String =
  ((~(((p0 * p1).toChar match { case '0' => p2; case '\u0000' => p3; case n if n.isDigit => p4; case _ => p5 })))).toString

@main def run(): Unit =
  show("0.f.v", ((~(((Consts.k0c0 * Consts.k0c1).toChar match { case '0' => Consts.k0c2; case '\u0000' => Consts.k0c3; case n if n.isDigit => Consts.k0c4; case _ => Consts.k0c5 })))).toString)
  show("0.r.v", r0v(63L, 3, 2147483647, (-1), 0, 255))
