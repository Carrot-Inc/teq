// The trigger of `double-text-subnormal` in proptests/src/known.rs, shrunk by tests/prop-known.sh:
// 2 of the program's lines differ, of 1 cases
// line 0.f.t: the targets print
//     interpreter: 1.0E-45
//     javascript: 1.401298464324817e-45
//     jvm: 1.4E-45
//     scalac: 1.4E-45
// folded: Consts.kc0
// at run time: Vars.kv0
//   where final val kc0 = 1.4E-45f
//   where var kv0: Float = 1.4E-45f
// line 0.r.t: the targets print
//     interpreter: 1.0E-45
//> using platform js
object Consts:
  final val none = 0
  final val k0c0 = 1.4E-45f

object Vars:
  var none: Int = 0
  var k0v0: Float = 1.4E-45f

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

def r0v(): String =
  bits((Vars.k0v0))

def r0t(): String =
  (if tie((Vars.k0v0)) then "tie" else (Vars.k0v0).toString)

@main def run(): Unit =
  show("0.f.v", bits((Consts.k0c0)))
  show("0.f.t", (if tie((Consts.k0c0)) then "tie" else (Consts.k0c0).toString))
  show("0.r.v", r0v())
  show("0.r.t", r0t())
