// The trigger of `js-conditional-condition` in proptests/src/known.rs, shrunk by tests/prop-known.sh:
// 2 of the program's lines differ, of 1 cases
// line 0.f.v: the targets print
//     interpreter: 0
//     javascript: -1
//     scalac: 0
// folded: (~((if (if Float.NaN.isNaN then false else (Float.NaN <= Consts.kc0)) then Float.PositiveInfinity else Consts.kc1.lastIndexOf("a\ud834\udd1eb").toFloat).toInt))
// at run time: (~((if (if Vars.kv0.isNaN then false else (Vars.kv0 <= p0)) then p1 else Vars.kv1.lastIndexOf(p2).toFloat).toInt))
//   where inline val kc0 = 99999999L
//   where final val kc1 = "\ud834\udd1e"
//   where var kv0: Float = Float.NaN
//   where var kv1: String = "\ud834\udd1e"
//   where p0: Long is 99999999L
object Consts:
  final val none = 0
  val written: 1 = 1
  inline val k0c0 = 99999999L
  final val k0c1 = "\ud834\udd1e"

object Vars:
  var none: Int = 0
  var k0v0: Float = Float.NaN
  var k0v1: String = "\ud834\udd1e"

def units(text: String): String =
  var out = ""
  var i = 0
  while i < text.length do
    out = out + text.charAt(i).toInt + " "
    i += 1
  out

def bits(x: Double): String =
  if x.isNaN then "NaN" else java.lang.Double.doubleToLongBits(x).toString

def bits(x: Float): String =
  if x.isNaN then "NaN" else java.lang.Float.floatToIntBits(x).toString

def plane(s: String): Boolean =
  var i = 0
  var ok = true
  while i < s.length do
    val u = s.charAt(i).toInt
    if u >= 55296 && u <= 57343 then ok = false
    i += 1
  ok

def space(c: Char): Boolean = c == ' ' || (c >= '\t' && c <= '\r')

def clean(s: String): Boolean =
  var i = 0
  var ok = true
  while i < s.length && s.charAt(i) <= ' ' do
    if !space(s.charAt(i)) then ok = false
    i += 1
  var j = s.length - 1
  while j >= i && s.charAt(j) <= ' ' do
    if !space(s.charAt(j)) then ok = false
    j -= 1
  ok

def span(s: String, a: Int, b: Int): Boolean = plane(s) && a >= 0 && a <= b && b <= s.length

def within(s: String, i: Int): Boolean = i >= 0 && i < s.length

def fits(a: Long, b: Long): Boolean =
  val m = Math.floorMod(a, b)
  if m < 0 then a <= 9223372036854775807L + m else a >= -9223372036854775807L - 1L + m

def tie(x: Float): Boolean =
  val t = x.toDouble.toString
  var digits = ""
  var i = 0
  while i < t.length && t.charAt(i) != 'E' && t.charAt(i) != 'e' do
    if t.charAt(i) >= '0' && t.charAt(i) <= '9' && (digits.length > 0 || t.charAt(i) != '0') then digits = digits + t.charAt(i)
    i += 1
  while digits.length > 0 && digits.charAt(digits.length - 1) == '0' do digits = digits.substring(0, digits.length - 1)
  digits.length >= 8 && digits.length <= 9 && digits.charAt(digits.length - 1) == '5'

def subnormal(x: Double): Boolean = !x.isNaN && x != 0.0 && Math.abs(x) < 2.2250738585072014E-308

def subnormal(x: Float): Boolean = !x.isNaN && x != 0.0f && Math.abs(x.toDouble) < 1.17549435E-38

def show(id: String, text: => String): Unit =
  val shown =
    try text
    catch
      case e: ArithmeticException => "ArithmeticException"
      case e: IndexOutOfBoundsException => "IndexOutOfBoundsException"
      case e: NumberFormatException => "NumberFormatException"
  println(id + ":" + shown)

def r0v(p0: Long, p1: Float, p2: String): String =
  ((~((if (if Vars.k0v0.isNaN then false else (Vars.k0v0 <= p0)) then p1 else Vars.k0v1.lastIndexOf(p2).toFloat).toInt))).toString

@main def run(): Unit =
  show("0.f.v", ((~((if (if Float.NaN.isNaN then false else (Float.NaN <= Consts.k0c0)) then Float.PositiveInfinity else Consts.k0c1.lastIndexOf("a\ud834\udd1eb").toFloat).toInt))).toString)
  show("0.r.v", r0v(99999999L, Float.PositiveInfinity, "a\ud834\udd1eb"))
