// The trigger of `js-int-remainder-negative-zero` in proptests/src/known.rs, shrunk by tests/prop-known.sh:
// 1 of the program's lines differ, of 1 cases
// line 0.f.v: on javascript the folded form prints "0" and the other "-9223372036854775808"
// folded: (if Consts.kc0 then (if (!(Consts.kc1)) then (127: Byte) else (3 match { case -11251335 => Consts.kc2; case 65535 => Consts.kc3; case n if n % 2 == 0 => 2L; case _ => Consts.kc4 }).toByte).toDouble else (-(((-2147483647) % 2147483647).toDouble)))
// at run time: (if Vars.kv0 then (if (!(p0)) then p1 else (Vars.kv1 match { case -11251335 => Vars.kv2; case 65535 => Vars.kv3; case n if n % 2 == 0 => Vars.kv4; case _ => p2 }).toByte).toDouble else (-((p3 % Vars.kv5).toDouble)))
//   where inline val kc0 = false
//   where final val kc1 = false
//   where inline val kc2 = 65L
//   where final val kc3 = 2L
//   where inline val kc4 = 2147483647L
//   where var kv0: Boolean = false
//   where var kv1: Int = 3
//   where var kv2: Long = 65L
//> using platform js
object Consts:
  final val none = 0
  val written: 1 = 1
  inline val k0c0 = false
  final val k0c1 = false
  inline val k0c2 = 65L
  final val k0c3 = 2L
  inline val k0c4 = 2147483647L

object Vars:
  var none: Int = 0
  var k0v0: Boolean = false
  var k0v1: Int = 3
  var k0v2: Long = 65L
  var k0v3: Long = 2L
  var k0v4: Long = 2L
  var k0v5: Int = 2147483647

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

def r0v(p0: Boolean, p1: Byte, p2: Long, p3: Int): String =
  java.lang.Double.doubleToLongBits(((if Vars.k0v0 then (if (!(p0)) then p1 else (Vars.k0v1 match { case -11251335 => Vars.k0v2; case 65535 => Vars.k0v3; case n if n % 2 == 0 => Vars.k0v4; case _ => p2 }).toByte).toDouble else (-((p3 % Vars.k0v5).toDouble))))).toString

def r0t(p0: Boolean, p1: Byte, p2: Long, p3: Int): String =
  ((if Vars.k0v0 then (if (!(p0)) then p1 else (Vars.k0v1 match { case -11251335 => Vars.k0v2; case 65535 => Vars.k0v3; case n if n % 2 == 0 => Vars.k0v4; case _ => p2 }).toByte).toDouble else (-((p3 % Vars.k0v5).toDouble)))).toString

@main def run(): Unit =
  show("0.f.v", java.lang.Double.doubleToLongBits(((if Consts.k0c0 then (if (!(Consts.k0c1)) then (127: Byte) else (3 match { case -11251335 => Consts.k0c2; case 65535 => Consts.k0c3; case n if n % 2 == 0 => 2L; case _ => Consts.k0c4 }).toByte).toDouble else (-(((-2147483647) % 2147483647).toDouble))))).toString)
  show("0.f.t", ((if Consts.k0c0 then (if (!(Consts.k0c1)) then (127: Byte) else (3 match { case -11251335 => Consts.k0c2; case 65535 => Consts.k0c3; case n if n % 2 == 0 => 2L; case _ => Consts.k0c4 }).toByte).toDouble else (-(((-2147483647) % 2147483647).toDouble)))).toString)
  show("0.r.v", r0v(false, (127: Byte), 2147483647L, (-2147483647)))
  show("0.r.t", r0t(false, (127: Byte), 2147483647L, (-2147483647)))
